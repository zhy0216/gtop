use gpui::*;
use gpui_component::{
    ActiveTheme,
    menu::PopupMenuItem,
    table::{Column, ColumnSort, TableDelegate, TableState},
};
use sysinfo::{Signal, System};

use crate::formatting::format_bytes;
use crate::models::{ProcessInfo, ProcessSortField};

pub struct ProcessTableDelegate {
    pub all_processes: Vec<ProcessInfo>,
    pub filtered_processes: Vec<ProcessInfo>,
    filter_text: String,
    columns: Vec<Column>,
    sort_field: ProcessSortField,
    sort_order: ColumnSort,
}

impl ProcessTableDelegate {
    pub fn new() -> Self {
        Self {
            all_processes: Vec::new(),
            filtered_processes: Vec::new(),
            filter_text: String::new(),
            columns: vec![
                Column::new("pid", "PID").width(70.).sortable(),
                Column::new("name", "Name").width(300.).sortable(),
                Column::new("cpu", "CPU %")
                    .width(80.)
                    .sortable()
                    .sort(ColumnSort::Descending),
                Column::new("memory", "Memory").width(100.).sortable(),
            ],
            sort_field: ProcessSortField::Cpu,
            sort_order: ColumnSort::Descending,
        }
    }

    pub fn update_processes(&mut self, sys: &System) {
        self.all_processes.clear();
        self.all_processes
            .extend(sys.processes().iter().map(|(pid, process)| ProcessInfo {
                pid: *pid,
                name: process.name().to_string_lossy().to_string(),
                cpu_usage: process.cpu_usage(),
                memory: process.memory(),
            }));

        self.apply_filter_and_sort();
    }

    pub fn set_filter(&mut self, text: String) {
        self.filter_text = text;
        self.apply_filter_and_sort();
    }

    fn apply_filter_and_sort(&mut self) {
        self.filtered_processes.clear();
        if self.filter_text.is_empty() {
            self.filtered_processes
                .extend(self.all_processes.iter().cloned());
        } else {
            let filter = self.filter_text.to_lowercase();
            self.filtered_processes.extend(
                self.all_processes
                    .iter()
                    .filter(|p| {
                        p.name.to_lowercase().contains(&filter)
                            || p.pid.as_u32().to_string().contains(&filter)
                    })
                    .cloned(),
            );
        }

        self.sort_processes();
    }

    fn sort_processes(&mut self) {
        let is_descending = matches!(self.sort_order, ColumnSort::Descending);

        match self.sort_field {
            ProcessSortField::Pid => {
                self.filtered_processes.sort_by(|a, b| {
                    let cmp = a.pid.as_u32().cmp(&b.pid.as_u32());
                    if is_descending { cmp.reverse() } else { cmp }
                });
            }
            ProcessSortField::Name => {
                self.filtered_processes.sort_by(|a, b| {
                    let cmp = a.name.to_lowercase().cmp(&b.name.to_lowercase());
                    if is_descending { cmp.reverse() } else { cmp }
                });
            }
            ProcessSortField::Cpu => {
                self.filtered_processes.sort_by(|a, b| {
                    let cmp = a
                        .cpu_usage
                        .partial_cmp(&b.cpu_usage)
                        .unwrap_or(std::cmp::Ordering::Equal);
                    if is_descending { cmp.reverse() } else { cmp }
                });
            }
            ProcessSortField::Memory => {
                self.filtered_processes.sort_by(|a, b| {
                    let cmp = a.memory.cmp(&b.memory);
                    if is_descending { cmp.reverse() } else { cmp }
                });
            }
        }
    }
}

impl TableDelegate for ProcessTableDelegate {
    fn columns_count(&self, _cx: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _cx: &App) -> usize {
        self.filtered_processes.len()
    }

    fn column(&self, col_ix: usize, _cx: &App) -> Column {
        self.columns[col_ix].clone()
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let Some(process) = self.filtered_processes.get(row_ix) else {
            return div().into_any_element();
        };

        match col_ix {
            0 => div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(format!("{}", process.pid))
                .into_any_element(),
            1 => div()
                .text_sm()
                .text_color(cx.theme().foreground)
                .truncate()
                .child(process.name.clone())
                .into_any_element(),
            2 => div()
                .text_xs()
                .text_color(if process.cpu_usage > 50.0 {
                    cx.theme().red
                } else if process.cpu_usage > 20.0 {
                    cx.theme().yellow
                } else {
                    cx.theme().blue
                })
                .child(format!("{:.1}%", process.cpu_usage))
                .into_any_element(),
            3 => div()
                .text_xs()
                .text_color(cx.theme().green)
                .child(format_bytes(process.memory))
                .into_any_element(),
            _ => div().into_any_element(),
        }
    }

    fn perform_sort(
        &mut self,
        col_ix: usize,
        sort: ColumnSort,
        _window: &mut Window,
        _cx: &mut Context<TableState<Self>>,
    ) {
        self.sort_order = sort;
        self.sort_field = match col_ix {
            0 => ProcessSortField::Pid,
            1 => ProcessSortField::Name,
            2 => ProcessSortField::Cpu,
            3 => ProcessSortField::Memory,
            _ => ProcessSortField::Cpu,
        };
        self.sort_processes();
    }

    fn context_menu(
        &mut self,
        row_ix: usize,
        menu: gpui_component::menu::PopupMenu,
        _window: &mut Window,
        _cx: &mut Context<TableState<Self>>,
    ) -> gpui_component::menu::PopupMenu {
        if let Some(process) = self.filtered_processes.get(row_ix) {
            let pid = process.pid;
            let name = process.name.clone();
            menu.item(
                PopupMenuItem::new(format!("Kill \"{}\" (PID {})", name, pid)).on_click(
                    move |_, _window, _cx| {
                        let mut sys = System::new();
                        sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
                        if let Some(process) = sys.process(pid) {
                            let _ = process.kill_with(Signal::Term);
                        }
                    },
                ),
            )
        } else {
            menu
        }
    }
}
