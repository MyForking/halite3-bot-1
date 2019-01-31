use imgui::*;

mod support_gfx;

const CLEAR_COLOR: [f32; 4] = [0.01, 0.02, 0.03, 1.0];

fn main() {
    support_gfx::run("hello_gfx.rs".to_owned(), CLEAR_COLOR, hello_world);
}

fn hello_world(ui: &Ui) -> bool {
    ui.window(im_str!("Hello world"))
        .size((300.0, 100.0), ImGuiCond::FirstUseEver)
        .build(|| {
            ui.text(im_str!("Hello world!"));
            ui.text(im_str!("こんにちは世界！"));
            ui.text(im_str!("This...is...imgui-rs!"));
            ui.separator();
            let mouse_pos = ui.imgui().mouse_pos();
            ui.text(im_str!(
                "Mouse Position: ({:.1},{:.1})",
                mouse_pos.0,
                mouse_pos.1
            ));

            let mut open = true;
            ui.show_demo_window(&mut open);
        });

    true
}
