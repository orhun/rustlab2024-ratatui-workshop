# Effects

Back in the old days, MSN messenger had this nudge feature where you could send a nudge to your friend's chat window. Let's recreate this feature in our TUI with some terminal effects!

![effects](images/effects.gif)

## Creating Effects

This is possible to thanks to the [`tachyonfx`](https://github.com/junkdog/tachyonfx), a shader-like effects library for Ratatui. So we can start off by adding this crate to our dependencies:

```sh
cargo add tachyonfx@0.7.0
```

The effects are rendered on top of the current buffer, so they essentially need another layer. We can simply re-use the `popup` module for them :)

In `src/popup.rs`:

```diff
 use ratatui_explorer::{FileExplorer, Theme};
 use ratatui_image::{picker::Picker, protocol::StatefulProtocol, StatefulImage};
+use tachyonfx::{
+    fx::{self, Direction as FxDirection},
+    Duration, Effect, EffectRenderer, EffectTimer, Interpolation, Shader,
+};
 use tokio::sync::mpsc::UnboundedSender;
 use tui_textarea::{Input, Key};

@@ -20,6 +24,7 @@ pub enum Popup {
     FileExplorer(FileExplorer, UnboundedSender<Event>),
     ImagePreview(Box<dyn StatefulProtocol>, UnboundedSender<Event>),
     MarkdownPreview(String, UnboundedSender<Event>),
+    Effect(Effect, UnboundedSender<Event>),
 }

 impl Popup {
@@ -64,6 +69,31 @@ impl Popup {
         ))
     }

+    pub fn effect(event_sender: UnboundedSender<Event>) -> Self {
+        let effect = fx::sequence(&[
+            fx::ping_pong(fx::sweep_in(
+                FxDirection::DownToUp,
+                10,
+                0,
+                Color::DarkGray,
+                EffectTimer::from_ms(3000, Interpolation::QuadIn),
+            )),
+            fx::hsl_shift_fg([360.0, 0.0, 0.0], 750),
+            fx::hsl_shift_fg([0.0, -100.0, 0.0], 750),
+            fx::hsl_shift_fg([0.0, -100.0, 0.0], 750).reversed(),
+            fx::hsl_shift_fg([0.0, 100.0, 0.0], 750),
+            fx::hsl_shift_fg([0.0, 100.0, 0.0], 750).reversed(),
+            fx::hsl_shift_fg([0.0, 0.0, -100.0], 750),
+            fx::hsl_shift_fg([0.0, 0.0, -100.0], 750).reversed(),
+            fx::hsl_shift_fg([0.0, 0.0, 100.0], 750),
+            fx::hsl_shift_fg([0.0, 0.0, 100.0], 750).reversed(),
+            fx::dissolve((800, Interpolation::SineOut)),
+            fx::coalesce((800, Interpolation::SineOut)),
+        ]);
+
+        Popup::Effect(effect, event_sender)
+    }
+
     pub async fn handle_input(
         &mut self,
         input: Input,
@@ -109,6 +139,14 @@ impl Widget for &mut Popup {
             Popup::MarkdownPreview(contents, _) => {
                 render_markdown_preview(area, buf, contents);
             }
+            Popup::Effect(effect, event_sender) => {
+                if effect.running() {
+                    render_effect(area, buf, effect);
+                    let _ = event_sender.send(Event::EffectRendered);
+                } else {
+                    let _ = event_sender.send(Event::PopupClosed);
+                }
+            }
         }
     }
 }
@@ -149,6 +187,11 @@ fn render_markdown_preview(area: Rect, buf: &mut Buffer, contents: &str) {
     text.render(popup_area.offset(Offset { x: 1, y: 1 }), buf);
 }

+fn render_effect(area: Rect, buf: &mut Buffer, effect: &mut Effect) {
+    let popup_area = popup_area(area, 100, 100);
+    buf.render_effect(effect, popup_area, Duration::from_millis(10));
+}
+
 fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
     let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
     let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
```

Here we added a new variant to the `Popup` enum called `Effect` (which contains the effect) and we are no longer using the `Clear` widget to clear the area. But the most important part is this:

```rust
let effect = fx::sequence(&[
    fx::ping_pong(fx::sweep_in(
        FxDirection::DownToUp,
        10,
        0,
        Color::DarkGray,
        EffectTimer::from_ms(3000, Interpolation::QuadIn),
    )),
    fx::hsl_shift_fg([360.0, 0.0, 0.0], 750),
    fx::hsl_shift_fg([0.0, -100.0, 0.0], 750),
    fx::hsl_shift_fg([0.0, -100.0, 0.0], 750).reversed(),
    fx::hsl_shift_fg([0.0, 100.0, 0.0], 750),
    fx::hsl_shift_fg([0.0, 100.0, 0.0], 750).reversed(),
    fx::hsl_shift_fg([0.0, 0.0, -100.0], 750),
    fx::hsl_shift_fg([0.0, 0.0, -100.0], 750).reversed(),
    fx::hsl_shift_fg([0.0, 0.0, 100.0], 750),
    fx::hsl_shift_fg([0.0, 0.0, 100.0], 750).reversed(),
    fx::dissolve((800, Interpolation::SineOut)),
    fx::coalesce((800, Interpolation::SineOut)),
]);
```

That's `tachyonfx` doing its magic and constructing an effect to apply to the buffer. You can take a look at the [examples](https://github.com/junkdog/tachyonfx/tree/development/examples) to come up with your own composition of effects.

## Handling Events

If you have realized, we are sending an `Event::EffectRendered` when the effect is running. This is because we need to redraw the effect on the next frame.

So we need to handle this event in `src/app.rs`:

```diff
pub enum Event {
     Terminal(CrosstermEvent),
     FileSelected(File),
     PopupClosed,
+    EffectRendered,
 }

 impl From<CrosstermEvent> for Event {
@@ -126,6 +127,7 @@ impl App {
             Event::PopupClosed => {
                 self.popup = None;
             }
+            Event::EffectRendered => {}
         }
         Ok(())
     }
@@ -241,7 +243,11 @@ impl App {
                     self.send(Command::ListUsers).await;
                 }
             }
-            RoomEvent::Nudge(_) => {}
+            RoomEvent::Nudge(username) => {
+                if username == self.message_list.username {
+                    self.popup = Some(Popup::effect(self.event_sender.clone()));
+                }
+            }
             RoomEvent::File { .. } => {}
         }
     }
```

We don't need to do anything to handle the `EffectRendered` event, it is enough to just receive it on the application side so the render loop can continue.

Also, we are setting the `Popup::Effect` variant when a nudge event is received for the current user.

## Updating the UI

As a final touch, we can update the `src/message_list.rs` to display a message when a nudge is received:

```diff
impl MessageList {
                 " sent a file: ".into(),
                 Span::from(filename).red().magenta(),
             ])),
-            _ => None,
+            RoomEvent::Nudge(name) => Some(Line::from(vec![
+                date.italic(),
+                " | ".into(),
+                Span::from(username).cyan(),
+                " nudged ".into(),
+                Span::from(name).green().italic(),
+            ])),
         }
     }
 }
```
