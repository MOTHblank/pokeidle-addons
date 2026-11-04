//! Reusable automation labels, badges, statistics and resource chips.
use super::{compact_text, egui, tr, Align, FontId, Layout, Margin, RichText, Stroke,
    ACCENT, BORDER, DIM, GOOD, MUTED, PANEL, PANEL_ALT, TEXT};

// Detailed view uses close label/value pairs instead of pushing numbers to the
// far edge of large columns. Preserve full values in the hover tooltip.
pub(super) fn detail_stat_line(ui: &mut egui::Ui, label: &str, value: &str) {
    // On a narrow workspace, a fixed 108px label column leaves little room
    // for the value. Stack the same shared presentation instead of clipping
    // the value or widening its containing card beyond the viewport.
    if ui.available_width() < 310.0 {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.label(RichText::new(tr(label)).size(10.0).color(DIM));
            ui.add(egui::Label::new(
                RichText::new(value).size(11.0).strong().color(TEXT),
            ).wrap()).on_hover_text(value);
        });
    } else {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            ui.add_sized(
                [108.0, 18.0],
                egui::Label::new(RichText::new(tr(label)).size(10.0).color(DIM))
                    .truncate(),
            ).on_hover_text(tr(label));
            ui.add(
                egui::Label::new(
                    RichText::new(compact_text(value, 32)).size(11.0).strong().color(TEXT),
                ).truncate(),
            ).on_hover_text(value);
        });
    }
}

pub(super) fn resource_chip(ui: &mut egui::Ui, text: &str) {
    egui::Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(7.0)
        .inner_margin(Margin::symmetric(9, 6))
        .show(ui, |ui| {
            ui.label(RichText::new(compact_text(text, 24)).size(10.0).color(TEXT))
                .on_hover_text(text);
        });
}

pub(super) fn automation_badge(ui: &mut egui::Ui, label: &str, value: &str, good: bool) -> egui::Response {
    let max_chars = if ui.available_width() < 240.0 { 16 }
        else if ui.available_width() < 340.0 { 24 }
        else { 54 };
    egui::Frame::new()
        .fill(if good { GOOD.linear_multiply(0.08) } else { PANEL_ALT })
        .stroke(Stroke::new(1.0, if good { GOOD.linear_multiply(0.28) } else { BORDER }))
        .corner_radius(7.0)
        .inner_margin(Margin::symmetric(9, 6))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("● {}", tr(label)))
                        .size(9.0)
                        .strong()
                        .color(if good { GOOD } else { MUTED }),
                );
                ui.add_space(4.0);
                ui.label(RichText::new(compact_text(tr(value), max_chars)).size(9.0).color(TEXT))
                    .on_hover_text(tr(value));
            });
        }).response
}

pub(super) fn bonus_chip(ui: &mut egui::Ui, text: &str) {
    let display = compact_text(text, 38);
    let measured = ui
        .painter()
        .layout_no_wrap(display.clone(), FontId::proportional(10.0), TEXT)
        .size();

    // Reserve the whole chip width before laying it out in a wrapping row.
    // Otherwise egui can give a chip only the remaining few pixels in the row,
    // causing each character to wrap vertically.
    let max_width = (ui.max_rect().width() - 20.0).max(44.0);
    let chip_width = (measured.x + 28.0).min(max_width);
    let text_width = (chip_width - 20.0).max(24.0);
    let chip_height = measured.y + 14.0;

    ui.allocate_ui_with_layout(
        egui::vec2(chip_width, chip_height),
        Layout::left_to_right(Align::Center),
        |chip_ui| {
            egui::Frame::new()
                .fill(ACCENT.linear_multiply(0.10))
                .stroke(Stroke::new(1.0, ACCENT.linear_multiply(0.28)))
                .corner_radius(7.0)
                .inner_margin(Margin::symmetric(9, 6))
                .show(chip_ui, |content_ui| {
                    content_ui.add_sized(
                        egui::vec2(text_width, measured.y),
                        egui::Label::new(
                            RichText::new(display)
                                .size(10.0)
                                .strong()
                                .color(TEXT),
                        )
                        .truncate(),
                    );
                });
        },
    );
}


