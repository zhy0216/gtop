use std::collections::VecDeque;
use std::time::{Duration, Instant};

use gpui::{prelude::FluentBuilder as _, *};
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable, TitleBar,
    chart::AreaChart,
    h_flex,
    input::{Input, InputEvent, InputState},
    progress::Progress,
    tab::{Tab, TabBar},
    table::{DataTable, TableState},
    v_flex,
};
use smol::Timer;
use sysinfo::{Components, Disks, Networks, System};

use crate::formatting::{format_rate, format_uptime};
use crate::models::*;
use crate::process_table::ProcessTableDelegate;

const INTERVAL: Duration = Duration::from_millis(3000);
const MAX_DATA_POINTS: usize = 120;
const TAB_FADE_DURATION: Duration = Duration::from_millis(200);

pub struct SystemMonitor {
    sys: System,
    disks: Disks,
    networks: Networks,
    components: Components,
    data: VecDeque<MetricPoint>,
    time_index: usize,
    active_tab: MonitorTab,
    tab_switch_counter: usize,
    process_table: Entity<TableState<ProcessTableDelegate>>,
    disk_info: Vec<DiskInfo>,
    battery_info: Vec<BatteryInfo>,
    cpu_cores: Vec<CpuCoreUsage>,
    cpu_temp: Option<f32>,
    net_stats: NetworkStats,
    process_filter: String,
    filter_input: Entity<InputState>,
    // Display values
    display_cpu: f64,
    display_memory: f64,
    // Network rate tracking
    last_collect_time: Instant,
}

impl SystemMonitor {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();

        let disks = Disks::new_with_refreshed_list();
        let networks = Networks::new_with_refreshed_list();
        let components = Components::new_with_refreshed_list();

        let process_delegate = ProcessTableDelegate::new();
        let process_table = cx.new(|cx| {
            TableState::new(process_delegate, window, cx)
                .col_selectable(false)
                .col_movable(false)
        });

        let filter_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Search processes..."));

        cx.subscribe(
            &filter_input,
            |this: &mut Self, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    let text = this.filter_input.read(cx).value();
                    this.on_filter_changed(&text, cx);
                }
            },
        )
        .detach();

        let mut monitor = Self {
            sys,
            disks,
            networks,
            components,
            data: VecDeque::with_capacity(MAX_DATA_POINTS),
            time_index: 0,
            active_tab: MonitorTab::System,
            tab_switch_counter: 0,
            process_table,
            disk_info: Vec::new(),
            battery_info: Vec::new(),
            cpu_cores: Vec::new(),
            cpu_temp: None,
            net_stats: NetworkStats {
                rx_bytes_per_sec: 0,
                tx_bytes_per_sec: 0,
            },
            process_filter: String::new(),
            filter_input,
            display_cpu: 0.0,
            display_memory: 0.0,
            last_collect_time: Instant::now(),
        };

        monitor.collect_metrics(cx);

        // Data collection loop
        cx.spawn(async move |this, cx| {
            loop {
                Timer::after(INTERVAL).await;
                let result = this.update(cx, |this, cx| {
                    this.collect_metrics(cx);
                    cx.notify();
                });
                if result.is_err() {
                    break;
                }
            }
        })
        .detach();

        monitor
    }

    fn smoothed_metrics(&self) -> SmoothedMetrics {
        let (disk_used, disk_total) =
            self.disk_info
                .iter()
                .fold((0u64, 0u64), |(used, total), d| {
                    (used + d.used, total + d.total)
                });

        let total_swap = self.sys.total_swap() as f64;
        let used_swap = self.sys.used_swap() as f64;
        let swap_percent = if total_swap > 0.0 {
            (used_swap / total_swap * 100.0).min(100.0)
        } else {
            0.0
        };

        SmoothedMetrics {
            cpu: self.display_cpu,
            memory: self.display_memory,
            swap_percent,
            disk_used,
            disk_total,
        }
    }

    fn collect_metrics(&mut self, cx: &mut Context<Self>) {
        let elapsed = self.last_collect_time.elapsed();
        let elapsed_secs = elapsed.as_secs_f64().max(0.1);

        self.sys.refresh_cpu_all();
        self.sys.refresh_memory();
        if self.active_tab == MonitorTab::Processes {
            self.sys
                .refresh_processes(sysinfo::ProcessesToUpdate::All, true);
        }
        self.disks.refresh(true);
        self.networks.refresh(true);
        self.components.refresh(true);

        // CPU
        let cpu_usage = self.sys.global_cpu_usage() as f64;

        // Per-core CPU
        self.cpu_cores = self
            .sys
            .cpus()
            .iter()
            .map(|cpu| CpuCoreUsage {
                usage: cpu.cpu_usage(),
            })
            .collect();

        // Memory
        let total_memory = self.sys.total_memory() as f64;
        let used_memory = self.sys.used_memory() as f64;
        let memory_usage = if total_memory > 0.0 {
            (used_memory / total_memory * 100.0).min(100.0)
        } else {
            0.0
        };

        // Network — compute actual rate using elapsed time
        let mut rx_total: u64 = 0;
        let mut tx_total: u64 = 0;
        for data in self.networks.list().values() {
            rx_total += data.received();
            tx_total += data.transmitted();
        }
        self.net_stats = NetworkStats {
            rx_bytes_per_sec: (rx_total as f64 / elapsed_secs) as u64,
            tx_bytes_per_sec: (tx_total as f64 / elapsed_secs) as u64,
        };
        self.last_collect_time = Instant::now();

        // CPU temperature - find the first CPU-related sensor
        self.cpu_temp = self
            .components
            .list()
            .iter()
            .filter(|c| {
                let label = c.label().to_lowercase();
                label.contains("cpu") || label.contains("soc") || label.contains("die")
            })
            .find_map(|c| c.temperature());

        // Update display values directly
        self.display_cpu = cpu_usage;
        self.display_memory = memory_usage;

        let point = MetricPoint {
            time: format!("{}s", self.time_index),
            cpu: cpu_usage,
            memory: memory_usage,
        };

        if self.data.len() >= MAX_DATA_POINTS {
            self.data.pop_front();
        }
        self.data.push_back(point);
        self.time_index += 1;

        // Process table
        self.process_table.update(cx, |table, cx| {
            table.delegate_mut().update_processes(&self.sys);
            cx.notify();
        });

        // Disks
        self.disk_info = self
            .disks
            .iter()
            .map(|disk| DiskInfo {
                total: disk.total_space(),
                used: disk.total_space() - disk.available_space(),
            })
            .collect();

        self.update_battery_info();
    }

    fn update_battery_info(&mut self) {
        self.battery_info.clear();

        if let Ok(manager) = battery::Manager::new()
            && let Ok(batteries) = manager.batteries()
        {
            for battery in batteries.flatten() {
                let icon = match battery.state() {
                    battery::State::Charging => IconName::BatteryCharging,
                    battery::State::Discharging => IconName::BatteryMedium,
                    battery::State::Full => IconName::BatteryFull,
                    battery::State::Empty => IconName::Battery,
                    _ => IconName::Battery,
                };

                self.battery_info.push(BatteryInfo {
                    icon,
                    percentage: battery.state_of_charge().value * 100.0,
                });
            }
        }
    }

    fn set_active_tab(&mut self, index: usize, _window: &mut Window, cx: &mut Context<Self>) {
        let new_tab = MonitorTab::from_index(index);
        if new_tab != self.active_tab {
            self.active_tab = new_tab;
            self.tab_switch_counter += 1;
            cx.notify();
        }
    }

    fn on_filter_changed(&mut self, text: &SharedString, cx: &mut Context<Self>) {
        self.process_filter = text.to_string();
        self.process_table.update(cx, |table, cx| {
            table.delegate_mut().set_filter(text.to_string());
            cx.notify();
        });
        cx.notify();
    }

    fn render_chart(
        &self,
        title: &str,
        data: Vec<MetricPoint>,
        value_fn: impl Fn(&MetricPoint) -> f64 + 'static,
        color: Hsla,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let radius = cx.theme().radius;
        v_flex()
            .min_h(px(180.))
            .flex_1()
            .gap_1()
            .rounded(radius)
            .border_1()
            .border_color(cx.theme().border.opacity(0.6))
            .bg(cx.theme().secondary.opacity(0.3))
            .overflow_hidden()
            .child(
                h_flex()
                    .justify_between()
                    .py_1p5()
                    .px_3()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(cx.theme().foreground)
                            .child(title.to_string()),
                    )
                    .child({
                        let current_value = data.last().map(&value_fn).unwrap_or(0.0);
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(color)
                            .child(format!("{:.1}%", current_value))
                    }),
            )
            .child(
                AreaChart::new(data)
                    .x(|d| d.time.clone())
                    .y(value_fn)
                    .stroke(color)
                    .fill(linear_gradient(
                        0.,
                        linear_color_stop(color.opacity(0.25), 1.),
                        linear_color_stop(cx.theme().background.opacity(0.05), 0.),
                    ))
                    .tick_margin(15),
            )
    }

    fn render_cpu_cores(&self, cx: &Context<Self>) -> impl IntoElement {
        let radius = cx.theme().radius;
        let core_count = self.cpu_cores.len();

        v_flex()
            .gap_1()
            .rounded(radius)
            .border_1()
            .border_color(cx.theme().border.opacity(0.6))
            .bg(cx.theme().secondary.opacity(0.3))
            .p_3()
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(cx.theme().foreground)
                    .pb_2()
                    .child(format!("CPU Cores ({})", core_count)),
            )
            .child(div().flex().flex_row().flex_wrap().gap_1().children(
                self.cpu_cores.iter().enumerate().map(|(i, core)| {
                    let intensity = core.usage / 100.0;
                    let color = hsla(
                        0.33 - intensity * 0.33, // green → red
                        0.75,
                        0.35 + intensity * 0.15,
                        1.0,
                    );

                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .w(px(52.))
                        .h(px(24.))
                        .rounded(px(4.))
                        .bg(color.opacity(0.15))
                        .border_1()
                        .border_color(color.opacity(0.3))
                        .text_xs()
                        .text_color(color)
                        .child(format!("{}: {:.0}%", i, core.usage))
                }),
            ))
    }

    fn render_network_stats(&self, cx: &Context<Self>) -> impl IntoElement {
        let radius = cx.theme().radius;

        h_flex()
            .gap_4()
            .rounded(radius)
            .border_1()
            .border_color(cx.theme().border.opacity(0.6))
            .bg(cx.theme().secondary.opacity(0.3))
            .px_3()
            .py_2()
            .child(
                h_flex()
                    .gap_2()
                    .flex_1()
                    .items_center()
                    .child(
                        Icon::new(IconName::ArrowDown)
                            .xsmall()
                            .text_color(hsla(0.58, 0.80, 0.50, 1.0)),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().foreground)
                            .child(format!(
                                "Down: {}",
                                format_rate(self.net_stats.rx_bytes_per_sec)
                            )),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .flex_1()
                    .items_center()
                    .child(
                        Icon::new(IconName::ArrowUp)
                            .xsmall()
                            .text_color(hsla(0.33, 0.75, 0.45, 1.0)),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().foreground)
                            .child(format!(
                                "Up: {}",
                                format_rate(self.net_stats.tx_bytes_per_sec)
                            )),
                    ),
            )
    }

    fn render_system_tab(&self, cx: &Context<Self>) -> impl IntoElement {
        let data: Vec<MetricPoint> = self.data.iter().cloned().collect();
        let cpu_color = hsla(0.33, 0.75, 0.45, 1.0);
        let mem_color = hsla(0.58, 0.80, 0.50, 1.0);
        let data_clone = data.clone();
        v_flex()
            .p_4()
            .gap_3()
            .flex_1()
            .child(self.render_chart("CPU Usage", data_clone, |d| d.cpu, cpu_color, cx))
            .child(self.render_cpu_cores(cx))
            .child(self.render_chart("Memory Usage", data, |d| d.memory, mem_color, cx))
            .child(self.render_network_stats(cx))
    }

    fn render_processes_tab(&self, cx: &Context<Self>) -> impl IntoElement {
        let process_count = self
            .process_table
            .read(cx)
            .delegate()
            .filtered_processes
            .len();
        let total_count = self.process_table.read(cx).delegate().all_processes.len();

        v_flex()
            .size_full()
            .child(
                h_flex()
                    .px_3()
                    .py_2()
                    .gap_3()
                    .items_center()
                    .border_b_1()
                    .border_color(cx.theme().border.opacity(0.5))
                    .child(
                        Input::new(&self.filter_input)
                            .prefix(
                                Icon::new(IconName::Search)
                                    .xsmall()
                                    .text_color(cx.theme().muted_foreground),
                            )
                            .cleanable(true)
                            .small()
                            .appearance(false),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .flex_shrink_0()
                            .child(if self.process_filter.is_empty() {
                                format!("{} processes", total_count)
                            } else {
                                format!("{} / {} processes", process_count, total_count)
                            }),
                    ),
            )
            .child(
                DataTable::new(&self.process_table)
                    .bordered(false)
                    .stripe(true)
                    .small(),
            )
    }

    fn render_status_bar(&self, cx: &Context<Self>) -> impl IntoElement {
        let metrics = self.smoothed_metrics();
        let primary_battery = self.battery_info.first();
        let uptime = System::uptime();
        let cpu_temp = self.cpu_temp;

        let disk_percent = if metrics.disk_total > 0 {
            (metrics.disk_used as f64 / metrics.disk_total as f64 * 100.0) as f32
        } else {
            0.0
        };

        h_flex()
            .px_4()
            .gap_4()
            .h_7()
            .text_xs()
            .items_center()
            .justify_between()
            .border_t_1()
            .border_color(cx.theme().border.opacity(0.5))
            .bg(cx.theme().background)
            .text_color(cx.theme().muted_foreground)
            .child(
                h_flex()
                    .gap_4()
                    .child(
                        h_flex()
                            .gap_1p5()
                            .items_center()
                            .child(format!("up {}", format_uptime(uptime))),
                    )
                    .when(!self.disk_info.is_empty(), |this| {
                        this.child(
                            h_flex()
                                .gap_1p5()
                                .items_center()
                                .child(Icon::new(IconName::HardDrive).xsmall())
                                .child(
                                    Progress::new("status-disk")
                                        .w_12()
                                        .h(px(3.))
                                        .value(disk_percent),
                                )
                                .child(
                                    div()
                                        .w(px(28.))
                                        .text_right()
                                        .child(format!("{:.0}%", disk_percent)),
                                ),
                        )
                    })
                    .child({
                        h_flex()
                            .gap_1p5()
                            .items_center()
                            .child(Icon::new(IconName::MemoryStick).xsmall())
                            .child(
                                Progress::new("status-mem")
                                    .w_12()
                                    .h(px(3.))
                                    .value(metrics.memory as f32),
                            )
                            .child(
                                div()
                                    .w(px(28.))
                                    .text_right()
                                    .child(format!("{:.0}%", metrics.memory)),
                            )
                    })
                    .when(self.sys.total_swap() > 0, |this| {
                        this.child(
                            h_flex()
                                .gap_1p5()
                                .items_center()
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .child("Swap"),
                                )
                                .child(
                                    Progress::new("status-swap")
                                        .w_12()
                                        .h(px(3.))
                                        .value(metrics.swap_percent as f32),
                                )
                                .child(
                                    div()
                                        .w(px(28.))
                                        .text_right()
                                        .child(format!("{:.0}%", metrics.swap_percent)),
                                ),
                        )
                    })
                    .child({
                        h_flex()
                            .gap_1p5()
                            .items_center()
                            .child(Icon::new(IconName::Cpu).xsmall())
                            .child(
                                Progress::new("status-cpu")
                                    .w_12()
                                    .h(px(3.))
                                    .value(metrics.cpu as f32),
                            )
                            .child(
                                div()
                                    .w(px(28.))
                                    .text_right()
                                    .child(format!("{:.0}%", metrics.cpu)),
                            )
                    })
                    .when_some(cpu_temp, |this, temp| {
                        this.child(
                            h_flex().gap_1p5().items_center().child(
                                div()
                                    .w(px(42.))
                                    .text_right()
                                    .text_color(if temp > 80.0 {
                                        cx.theme().red
                                    } else if temp > 60.0 {
                                        cx.theme().yellow
                                    } else {
                                        cx.theme().muted_foreground
                                    })
                                    .child(format!("{:.0}°C", temp)),
                            ),
                        )
                    })
                    .child({
                        h_flex()
                            .gap_1p5()
                            .w(px(170.))
                            .items_center()
                            .child(Icon::new(IconName::Network).xsmall())
                            .child(format!(
                                "↓{} ↑{}",
                                format_rate(self.net_stats.rx_bytes_per_sec),
                                format_rate(self.net_stats.tx_bytes_per_sec)
                            ))
                    }),
            )
            .child(div().when_some(primary_battery, |this, battery| {
                this.child(
                    h_flex()
                        .gap_1p5()
                        .items_center()
                        .child(Icon::new(battery.icon.clone()).xsmall())
                        .child(format!("{:.0}%", battery.percentage)),
                )
            }))
    }
}

impl Render for SystemMonitor {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let active_tab_index = self.active_tab as usize;
        let tab_anim_id = format!("tab-fade-{}", self.tab_switch_counter);

        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(
                TitleBar::new()
                    .child(
                        TabBar::new("monitor-tabs")
                            .mt(px(1.))
                            .segmented()
                            .px_0()
                            .py(px(2.))
                            .bg(cx.theme().title_bar)
                            .selected_index(active_tab_index)
                            .on_click(cx.listener(|this, ix: &usize, window, cx| {
                                this.set_active_tab(*ix, window, cx);
                            }))
                            .child(Tab::new().label("System"))
                            .child(Tab::new().label("Processes")),
                    )
                    .child(
                        div()
                            .mr_4()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!(
                                "{:.1} GB RAM",
                                self.sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0
                            )),
                    ),
            )
            .child(
                div()
                    .id("tab-content")
                    .flex_1()
                    .overflow_y_scroll()
                    .map(|this| match self.active_tab {
                        MonitorTab::System => this.child(
                            div()
                                .size_full()
                                .child(self.render_system_tab(cx))
                                .with_animation(
                                    ElementId::Name(tab_anim_id.clone().into()),
                                    Animation::new(TAB_FADE_DURATION).with_easing(ease_in_out),
                                    |el, delta| el.opacity(delta),
                                ),
                        ),
                        MonitorTab::Processes => this.child(
                            div()
                                .size_full()
                                .child(self.render_processes_tab(cx))
                                .with_animation(
                                    ElementId::Name(tab_anim_id.clone().into()),
                                    Animation::new(TAB_FADE_DURATION).with_easing(ease_in_out),
                                    |el, delta| el.opacity(delta),
                                ),
                        ),
                    }),
            )
            .child(self.render_status_bar(cx))
    }
}
