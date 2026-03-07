use eframe::egui::Ui;
use eframe::egui::{self, Color32};
use std::collections::VecDeque;

pub struct UiTerminal {
    history: VecDeque<String>,
    size: usize,
}

impl UiTerminal {
    pub fn new(size: usize) -> Self {
        Self {
            history: VecDeque::with_capacity(size),
            size,
        }
    }

    pub fn print(&mut self, text: &str) {
        if self.history.len() >= self.size {
            self.history.pop_front();
        }
        let text = text.to_string();

        #[cfg(debug_assertions)]
        println!("{}", text);

        self.history.push_back(text);
    }

    pub fn show(&self, ui: &mut Ui) {
        egui::Frame::NONE
            .fill(Color32::BLACK)
            .inner_margin(10.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.set_width(ui.available_width());
                            for line in &self.history {
                                ui.add(egui::Label::new(egui::RichText::new(line).monospace()));
                            }
                        });
                    });
            });
    }
}
