# iced-color-triangle

A hue ring and triangle color selector for Iced 0.14.

Retain `Selection` in application state and replace it on each change.
Convert it to `iced::Color` when applying the selection. Keeping `Selection`
retains the chosen hue at black, white, and gray.

```rust
use iced::{Color, Element};
use iced_color_triangle::{Selection, color_triangle};

#[derive(Clone)]
enum Message {
    Pick(Selection),
}

fn view(selection: Selection) -> Element<'static, Message> {
    color_triangle(selection, Message::Pick).into()
}
```

The widget defaults to 240×240 logical pixels. Change it with `.size(…)`,
`.width(…)`, or `.height(…)`, which accept any `iced::Length`. The wheel scales
to the largest centered square that fits, and input follows the drawing.
It preserves alpha.
The triangle rotates so its colored corner points toward the hue-ring handle.
Callers own presets, component sliders, and Apply/Cancel behavior.
The library enables no renderer or window-system backend.
Enable `canvas` and your preferred renderer/runtime features in your application.

Run the example with:

```sh
cargo run --example picker
```

Licensed under MIT OR Apache-2.0.
