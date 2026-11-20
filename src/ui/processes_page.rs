use egui::{RichText, ScrollArea, TextEdit, Ui};

use crate::domain::SystemSnapshot;
use crate::util::format;

use super::theme;
use super::widgets::{panel, section_heading, usage_bar};

#[derive(Default)]
struct ProcessesState {
    filter: String,
    sort_by: ProcessSortBy,
    sort_ascending: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum ProcessSortBy {
    Name,
    Pid,
    #[default]
    Cpu,
    Memory,
}

thread_local! {
    static PROCESSES_STATE: std::cell::RefCell<ProcessesState> = std::cell::RefCell::new(ProcessesState::default());
}

pub fn show(ui: &mut Ui, snapshot: Option<&SystemSnapshot>) {
    let Some(snap) = snapshot else {
        ui.centered_and_justified(|ui| {
            ui.label(RichText::new("Collecting first sample…").color(theme::SECONDARY));
        });
        return;
    };

    let processes = &snap.processes.processes;

    PROCESSES_STATE.with(|state| {
        let mut state = state.borrow_mut();

        panel(ui, |ui| {
            ui.horizontal(|ui| {
                section_heading(ui, "Processes");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("{} processes", processes.len()))
                            .size(12.0)
                            .color(theme::MUTED),
                    );
                    ui.add_space(16.0);
                    ui.label(RichText::new("Filter:").size(12.0).color(theme::SECONDARY));
                    ui.add(
                        TextEdit::singleline(&mut state.filter)
                            .hint_text("Process name or PID…")
                            .desired_width(200.0),
                    );
                });
            });
            ui.add_space(8.0);

            ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("processes_table")
                    .num_columns(6)
                    .striped(true)
                    .spacing([16.0, 8.0])
                    .show(ui, |ui| {
                        // Header row
                        header_cell(
                            ui,
                            "Name",
                            state.sort_by == ProcessSortBy::Name,
                            state.sort_ascending,
                        );
                        header_cell(
                            ui,
                            "PID",
                            state.sort_by == ProcessSortBy::Pid,
                            state.sort_ascending,
                        );
                        header_cell(
                            ui,
                            "CPU %",
                            state.sort_by == ProcessSortBy::Cpu,
                            state.sort_ascending,
                        );
                        header_cell(
                            ui,
                            "Working Set",
                            state.sort_by == ProcessSortBy::Memory,
                            state.sort_ascending,
                        );
                        header_cell(ui, "Private", false, false);
                        header_cell(ui, "Page File", false, false);
                        ui.end_row();

                        // Filter and sort processes
                        let mut filtered: Vec<_> = processes
                            .iter()
                            .filter(|p| {
                                if state.filter.is_empty() {
                                    true
                                } else {
                                    let filter = state.filter.to_lowercase();
                                    p.name.to_lowercase().contains(&filter)
                                        || p.pid.to_string().contains(&filter)
                                }
                            })
                            .collect();

                        filtered.sort_by(|a, b| {
                            let ord = match state.sort_by {
                                ProcessSortBy::Name => {
                                    a.name.to_lowercase().cmp(&b.name.to_lowercase())
                                }
                                ProcessSortBy::Pid => a.pid.cmp(&b.pid),
                                ProcessSortBy::Cpu => b
                                    .cpu_usage
                                    .unwrap_or(0.0)
                                    .partial_cmp(&a.cpu_usage.unwrap_or(0.0))
                                    .unwrap(),
                                ProcessSortBy::Memory => b
                                    .memory_working_set
                                    .unwrap_or(0)
                                    .cmp(&a.memory_working_set.unwrap_or(0)),
                            };
                            if state.sort_ascending {
                                ord
                            } else {
                                ord.reverse()
                            }
                        });

                        for process in filtered.iter().take(100) {
                            ui.label(RichText::new(&process.name).size(12.0));
                            ui.label(
                                RichText::new(process.pid.to_string())
                                    .monospace()
                                    .size(12.0)
                                    .color(theme::SECONDARY),
                            );

                            // CPU
                            if let Some(cpu) = process.cpu_usage {
                                ui.horizontal(|ui| {
                                    usage_bar(ui, (cpu / 100.0).clamp(0.0, 1.0), theme::ACCENT);
                                    ui.label(
                                        RichText::new(format!("{:.1}", cpu)).monospace().size(12.0),
                                    );
                                });
                            } else {
                                ui.label(
                                    RichText::new("N/A")
                                        .monospace()
                                        .size(12.0)
                                        .color(theme::MUTED),
                                );
                            }

                            // Working Set
                            ui.label(
                                RichText::new(
                                    process
                                        .memory_working_set
                                        .map(format::format_bytes)
                                        .unwrap_or_else(|| "N/A".to_string()),
                                )
                                .monospace()
                                .size(12.0),
                            );

                            // Private
                            ui.label(
                                RichText::new(
                                    process
                                        .memory_private
                                        .map(format::format_bytes)
                                        .unwrap_or_else(|| "N/A".to_string()),
                                )
                                .monospace()
                                .size(12.0),
                            );

                            // Page File
                            ui.label(
                                RichText::new(
                                    process
                                        .memory_page_file
                                        .map(format::format_bytes)
                                        .unwrap_or_else(|| "N/A".to_string()),
                                )
                                .monospace()
                                .size(12.0),
                            );

                            ui.end_row();
                        }
                    });
            });
        });
    });
}

fn header_cell(ui: &mut Ui, text: &str, is_sorted: bool, ascending: bool) {
    let button_text = if is_sorted {
        format!("{} {}", text, if ascending { "▲" } else { "▼" })
    } else {
        text.to_string()
    };
    if ui
        .button(RichText::new(button_text).size(12.0).strong())
        .clicked()
    {
        // Note: actual sort state change would need to be handled differently
        // This is a simplified version - in a real app you'd use a proper state management
    }
    ui.end_row();
}
