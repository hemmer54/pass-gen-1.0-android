use std::time::Duration;

#[cfg(target_os = "android")]
use std::borrow::Cow;

use crate::password_core::{self, GenerationMode, GeneratorSettings};
use gpui::{div, prelude::*, px, rgb, Context, MouseButton, Render, Window};
use gpui_mobile::{set_system_chrome, StatusBarContentStyle, SystemChromeStyle};

const FONT_FAMILY: &str = "Fixedsys Excelsior";
const TITLE_FONT_FAMILY: &str = "JH_Fallout";
const DARK_SURFACE: u32 = 0x10100F;
const DARK_HEADER: u32 = 0x101610;

pub struct PasswordApp {
    settings: GeneratorSettings,
    password: String,
    history: Vec<String>,
    show_history: bool,
    clipboard_auto_clear: bool,
    revealed: bool,
    status: String,
}

impl Default for PasswordApp {
    fn default() -> Self {
        Self {
            settings: GeneratorSettings::default(),
            password: String::new(),
            history: Vec::new(),
            show_history: false,
            clipboard_auto_clear: true,
            revealed: true,
            status: "Ready to generate a secure credential".into(),
        }
    }
}

impl PasswordApp {
    pub fn new() -> Self {
        let mut app = Self::default();
        app.generate();
        app
    }

    fn theme() -> Theme {
        Theme {
            surface: DARK_SURFACE,
            surface_low: 0x171614,
            surface_container: 0x211F1B,
            primary: 0x6CCB7B,
            on_primary: 0x08110B,
            primary_container: 0x1E3A28,
            on_primary_container: 0xD5E8D5,
            on_surface: 0xE6E1D8,
            on_surface_variant: 0xA39C91,
            outline: 0x4B6250,
            outline_variant: 0x2B3A2E,
            tertiary: 0xE0BF70,
            error: 0xE07A7A,
        }
    }

    fn generate(&mut self) {
        match password_core::generate(self.settings) {
            Ok(password) => {
                replace_secret(&mut self.password, password.clone());
                self.history.insert(0, password);
                while self.history.len() > 8 {
                    if let Some(old) = self.history.pop() {
                        drop_secret(old);
                    }
                }
                self.status = self.strength().to_string();
            }
            Err(error) => self.status = error.to_string(),
        }
    }

    fn strength(&self) -> &'static str {
        password_core::strength_label(self.entropy_bits())
    }

    fn entropy_bits(&self) -> f32 {
        password_core::entropy_bits(self.settings)
    }

    fn set_mode(&mut self, mode: GenerationMode) {
        self.settings.mode = mode;
        self.revealed = true;
        self.generate();
    }

    fn adjust_length(&mut self, amount: isize) {
        let length = (self.settings.length as isize + amount).clamp(8, 64) as usize;
        if length != self.settings.length {
            self.settings.length = length;
            self.generate();
        }
    }

    fn adjust_words(&mut self, amount: isize) {
        let words = (self.settings.words as isize + amount).clamp(3, 8) as usize;
        if words != self.settings.words {
            self.settings.words = words;
            self.generate();
        }
    }

    fn clear_history(&mut self) {
        for item in self.history.drain(..) {
            drop_secret(item);
        }
        self.status = "In-memory history cleared".into();
    }

    fn copy_current(&mut self, cx: &mut Context<PasswordApp>) {
        if self.password.is_empty() {
            return;
        }
        self.status = match copy_to_clipboard(&self.password) {
            Ok(()) => {
                if self.clipboard_auto_clear {
                    cx.spawn(async move |_, app| {
                        app.background_executor()
                            .timer(Duration::from_secs(30))
                            .await;
                        let _ = app.update(|_| clear_clipboard());
                    })
                    .detach();
                }
                "Copied; clipboard clears in 30 seconds".into()
            }
            Err(error) => format!("Could not copy password: {error}"),
        };
    }
}

impl Render for PasswordApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Self::theme();
        let mode = self.settings.mode;
        let password = if self.revealed {
            self.password.clone()
        } else {
            "•".repeat(self.password.chars().count())
        };
        let strength = self.strength();
        let entropy = self.entropy_bits();
        let strength_color = match strength {
            "Weak" => theme.error,
            "Moderate" => theme.tertiary,
            _ => theme.primary,
        };
        let history = self.history.clone();
        let status = self.status.clone();
        let history_open = self.show_history;
        let has_charset = self.settings.uppercase
            || self.settings.lowercase
            || self.settings.digits
            || self.settings.symbols;

        set_system_chrome(&SystemChromeStyle {
            status_bar_color: Some(DARK_HEADER),
            status_bar_style: StatusBarContentStyle::Light,
            navigation_bar_color: Some(theme.surface),
        });

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(theme.surface))
            .text_color(rgb(theme.on_surface))
            .font_family(FONT_FAMILY)
            .child(
                div()
                    .w_full()
                    .h(px(android_safe_top_inset().max(24.0)))
                    .bg(rgb(DARK_HEADER)),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .px_4()
                    .py_3()
                    .bg(rgb(DARK_HEADER))
                    .child(
                        div()
                            .text_xl()
                            .font_family(TITLE_FONT_FAMILY)
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(rgb(0xD5E8D5))
                            .child("PASS GEN 1.0"),
                    ),
            )
            .child(
                div()
                    .id("password-generator-scroll")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .gap_3()
                    .px_4()
                    .py_4()
                    .overflow_y_scroll()
                    .child(password_card(
                        &password,
                        self.revealed,
                        strength,
                        entropy,
                        strength_color,
                        theme,
                        cx,
                    ))
                    .child(mode_switch(mode, theme, cx))
                    .when(mode == GenerationMode::RandomPassword, |content| {
                        content.child(random_options(self.settings, has_charset, theme, cx))
                    })
                    .when(mode == GenerationMode::Passphrase, |content| {
                        content.child(passphrase_options(self.settings, theme, cx))
                    })
                    .child(clipboard_settings(self.clipboard_auto_clear, theme, cx))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .justify_between()
                            .pt_1()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(theme.on_surface_variant))
                                    .child(status),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .gap_2()
                                    .child(small_button(
                                        "Clear History",
                                        theme,
                                        cx.listener(|this, _, _, cx| {
                                            this.clear_history();
                                            cx.notify();
                                        }),
                                    ))
                                    .child(
                                        div()
                                            .px_3()
                                            .py_2()
                                            .rounded_lg()
                                            .text_xs()
                                            .border_1()
                                            .border_color(rgb(theme.outline_variant))
                                            .child(if history_open {
                                                "Hide history"
                                            } else {
                                                "History"
                                            })
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(|this, _, _, cx| {
                                                    this.show_history = !this.show_history;
                                                    cx.notify();
                                                }),
                                            ),
                                    ),
                            ),
                    )
                    .when(history_open, |content| {
                        content.child(history_card(history, self.revealed, theme, cx))
                    }),
            )
    }
}

#[derive(Clone, Copy)]
struct Theme {
    surface: u32,
    surface_low: u32,
    surface_container: u32,
    primary: u32,
    on_primary: u32,
    primary_container: u32,
    on_primary_container: u32,
    on_surface: u32,
    on_surface_variant: u32,
    outline: u32,
    outline_variant: u32,
    tertiary: u32,
    error: u32,
}

fn password_card(
    password: &str,
    revealed: bool,
    strength: &str,
    entropy: f32,
    strength_color: u32,
    theme: Theme,
    cx: &mut Context<PasswordApp>,
) -> impl IntoElement {
    let value = password.to_owned();
    div()
        .flex()
        .flex_col()
        .gap_3()
        .p_4()
        .rounded_xl()
        .border_1()
        .border_color(rgb(theme.primary))
        .bg(rgb(theme.surface_container))
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_xs()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(rgb(theme.primary))
                        .child("GENERATED CREDENTIAL"),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(theme.on_surface_variant))
                        .child(if revealed { "VISIBLE" } else { "HIDDEN" }),
                ),
        )
        .child(
            div()
                .p_3()
                .rounded_lg()
                .bg(rgb(theme.surface))
                .text_lg()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(rgb(theme.on_surface))
                .child(value),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .flex_1()
                        .h(px(6.0))
                        .rounded_full()
                        .bg(rgb(theme.outline_variant))
                        .child(
                            div()
                                .h(px(6.0))
                                .rounded_full()
                                .bg(rgb(strength_color))
                                .w(px((entropy.min(128.0) / 128.0 * 240.0).max(8.0))),
                        ),
                )
                .child(
                    div()
                        .text_xs()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(rgb(strength_color))
                        .child(strength.to_owned()),
                ),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .gap_2()
                .child(small_button(
                    if revealed { "Hide" } else { "Reveal" },
                    theme,
                    cx.listener(|this, _, _, cx| {
                        this.revealed = !this.revealed;
                        cx.notify();
                    }),
                ))
                .child(small_button(
                    "Copy",
                    theme,
                    cx.listener(|this, _, _, cx| {
                        this.copy_current(cx);
                        cx.notify();
                    }),
                ))
                .child(
                    div()
                        .flex_1()
                        .px_3()
                        .py_2()
                        .rounded_lg()
                        .bg(rgb(theme.primary))
                        .text_color(rgb(theme.on_primary))
                        .text_center()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .child("Generate")
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                this.generate();
                                cx.notify();
                            }),
                        ),
                ),
        )
}

fn mode_switch(
    mode: GenerationMode,
    theme: Theme,
    cx: &mut Context<PasswordApp>,
) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .gap_2()
        .p_1()
        .rounded_xl()
        .bg(rgb(theme.surface_low))
        .child(mode_button(
            "Random password",
            mode == GenerationMode::RandomPassword,
            theme,
            cx.listener(|this, _, _, cx| {
                this.set_mode(GenerationMode::RandomPassword);
                cx.notify();
            }),
        ))
        .child(mode_button(
            "Passphrase",
            mode == GenerationMode::Passphrase,
            theme,
            cx.listener(|this, _, _, cx| {
                this.set_mode(GenerationMode::Passphrase);
                cx.notify();
            }),
        ))
}

fn random_options(
    settings: GeneratorSettings,
    has_charset: bool,
    theme: Theme,
    cx: &mut Context<PasswordApp>,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(section_title("PASSWORD RECIPE", theme))
        .child(control_card(
            "Length",
            format!("{} characters", settings.length),
            theme,
            div()
                .flex()
                .flex_row()
                .gap_2()
                .child(round_button(
                    "−",
                    theme,
                    cx.listener(|this, _, _, cx| {
                        this.adjust_length(-4);
                        cx.notify();
                    }),
                ))
                .child(round_button(
                    "+",
                    theme,
                    cx.listener(|this, _, _, cx| {
                        this.adjust_length(4);
                        cx.notify();
                    }),
                )),
        ))
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .p_3()
                .rounded_lg()
                .border_1()
                .border_color(rgb(if has_charset {
                    theme.outline_variant
                } else {
                    theme.error
                }))
                .bg(rgb(theme.surface_low))
                .child(toggle_line(
                    "Uppercase",
                    "A–Z",
                    settings.uppercase,
                    theme,
                    cx.listener(|this, _, _, cx| {
                        this.settings.uppercase = !this.settings.uppercase;
                        this.generate();
                        cx.notify();
                    }),
                ))
                .child(toggle_line(
                    "Lowercase",
                    "a–z",
                    settings.lowercase,
                    theme,
                    cx.listener(|this, _, _, cx| {
                        this.settings.lowercase = !this.settings.lowercase;
                        this.generate();
                        cx.notify();
                    }),
                ))
                .child(toggle_line(
                    "Numbers",
                    "2–9",
                    settings.digits,
                    theme,
                    cx.listener(|this, _, _, cx| {
                        this.settings.digits = !this.settings.digits;
                        this.generate();
                        cx.notify();
                    }),
                ))
                .child(toggle_line(
                    "Symbols",
                    "! @ # %",
                    settings.symbols,
                    theme,
                    cx.listener(|this, _, _, cx| {
                        this.settings.symbols = !this.settings.symbols;
                        this.generate();
                        cx.notify();
                    }),
                ))
                .child(toggle_line(
                    "Exclude ambiguous",
                    "Remove lookalike characters",
                    settings.exclude_ambiguous,
                    theme,
                    cx.listener(|this, _, _, cx| {
                        this.settings.exclude_ambiguous = !this.settings.exclude_ambiguous;
                        this.generate();
                        cx.notify();
                    }),
                ))
                .child(toggle_line(
                    "Avoid repeats",
                    "Never repeat adjacent characters",
                    settings.avoid_repeats,
                    theme,
                    cx.listener(|this, _, _, cx| {
                        this.settings.avoid_repeats = !this.settings.avoid_repeats;
                        this.generate();
                        cx.notify();
                    }),
                )),
        )
}

fn passphrase_options(
    settings: GeneratorSettings,
    theme: Theme,
    cx: &mut Context<PasswordApp>,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(section_title("PASSPHRASE RECIPE", theme))
        .child(control_card(
            "Words",
            format!("{} words", settings.words),
            theme,
            div()
                .flex()
                .flex_row()
                .gap_2()
                .child(round_button("−", theme, cx.listener(|this, _, _, cx| {
                    this.adjust_words(-1);
                    cx.notify();
                })))
                .child(round_button("+", theme, cx.listener(|this, _, _, cx| {
                    this.adjust_words(1);
                    cx.notify();
                }))),
        ))
        .child(toggle_line("Separator", "Tap to cycle: hyphen / dot / space", settings.separator != ' ', theme, cx.listener(|this, _, _, cx| {
            this.settings.separator = match this.settings.separator {
                '-' => '.',
                '.' => ' ',
                _ => '-',
            };
            this.generate();
            cx.notify();
        })))
        .child(
            div()
                .p_3()
                .rounded_lg()
                .bg(rgb(theme.primary_container))
                .text_xs()
                .text_color(rgb(theme.on_primary_container))
                .child("Passphrases use an offline word list and secure OS randomness. They are easier to type and remember."),
        )
}

fn clipboard_settings(
    auto_clear: bool,
    theme: Theme,
    cx: &mut Context<PasswordApp>,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .p_3()
        .rounded_lg()
        .border_1()
        .border_color(rgb(theme.outline_variant))
        .bg(rgb(theme.surface_low))
        .child(section_title("CLIPBOARD SETTINGS", theme))
        .child(toggle_line(
            "Auto-clear clipboard",
            "Clear copied passwords after 30 seconds",
            auto_clear,
            theme,
            cx.listener(|this, _, _, cx| {
                this.clipboard_auto_clear = !this.clipboard_auto_clear;
                cx.notify();
            }),
        ))
}

fn history_card(
    history: Vec<String>,
    revealed: bool,
    theme: Theme,
    cx: &mut Context<PasswordApp>,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .child(section_title("RECENT • MEMORY ONLY", theme))
                .child(small_button(
                    "Clear",
                    theme,
                    cx.listener(|this, _, _, cx| {
                        this.clear_history();
                        cx.notify();
                    }),
                )),
        )
        .children(history.into_iter().enumerate().map(|(index, item)| {
            let display = if revealed {
                item
            } else {
                "•".repeat(item.chars().count())
            };
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_2()
                .p_3()
                .rounded_lg()
                .bg(rgb(theme.surface_low))
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(theme.on_surface_variant))
                        .child(format!("{:02}", index + 1)),
                )
                .child(div().flex_1().text_sm().child(display))
        }))
}

fn section_title(title: &str, theme: Theme) -> impl IntoElement {
    div()
        .text_xs()
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(rgb(theme.primary))
        .child(title.to_owned())
}

fn control_card(
    title: &str,
    value: String,
    theme: Theme,
    controls: impl IntoElement,
) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .p_3()
        .rounded_lg()
        .border_1()
        .border_color(rgb(theme.outline_variant))
        .bg(rgb(theme.surface_low))
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_sm()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .child(title.to_owned()),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(theme.on_surface_variant))
                        .child(value),
                ),
        )
        .child(controls)
}

fn toggle_line(
    title: &str,
    detail: &str,
    enabled: bool,
    theme: Theme,
    handler: impl Fn(&gpui::MouseDownEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .py_2()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(div().text_sm().child(title.to_owned()))
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(theme.on_surface_variant))
                        .child(detail.to_owned()),
                ),
        )
        .child(
            div()
                .px_2()
                .py_1()
                .rounded_lg()
                .border_1()
                .border_color(rgb(if enabled {
                    theme.primary
                } else {
                    theme.outline
                }))
                .bg(rgb(if enabled {
                    theme.primary_container
                } else {
                    theme.surface_container
                }))
                .text_xs()
                .text_color(rgb(if enabled {
                    theme.on_primary_container
                } else {
                    theme.on_surface_variant
                }))
                .child(if enabled { "ON" } else { "OFF" })
                .on_mouse_down(MouseButton::Left, handler),
        )
        .on_mouse_down(MouseButton::Left, |_, _, _| {})
}

fn mode_button(
    title: &str,
    selected: bool,
    theme: Theme,
    handler: impl Fn(&gpui::MouseDownEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .flex_1()
        .px_3()
        .py_2()
        .rounded_lg()
        .bg(rgb(if selected {
            theme.primary_container
        } else {
            theme.surface_container
        }))
        .text_center()
        .text_sm()
        .text_color(rgb(if selected {
            theme.on_primary_container
        } else {
            theme.on_surface_variant
        }))
        .child(title.to_owned())
        .on_mouse_down(MouseButton::Left, handler)
}

fn round_button(
    title: &str,
    theme: Theme,
    handler: impl Fn(&gpui::MouseDownEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .w(px(38.0))
        .h(px(34.0))
        .rounded_lg()
        .bg(rgb(theme.primary_container))
        .text_color(rgb(theme.on_primary_container))
        .text_center()
        .text_lg()
        .child(title.to_owned())
        .on_mouse_down(MouseButton::Left, handler)
}

fn small_button(
    title: &str,
    theme: Theme,
    handler: impl Fn(&gpui::MouseDownEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .px_3()
        .py_2()
        .rounded_lg()
        .border_1()
        .border_color(rgb(theme.outline))
        .bg(rgb(theme.surface_container))
        .text_xs()
        .text_center()
        .child(title.to_owned())
        .on_mouse_down(MouseButton::Left, handler)
}

fn replace_secret(target: &mut String, value: String) {
    overwrite_string(target);
    *target = value;
}

fn drop_secret(mut value: String) {
    overwrite_string(&mut value);
}

fn overwrite_string(value: &mut String) {
    unsafe {
        for byte in value.as_bytes_mut() {
            *byte = 0;
        }
    }
    value.clear();
}

fn copy_to_clipboard(text: &str) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        use ::jni::objects::{JObject, JValue};
        use gpui_mobile::android::jni as mobile_jni;

        return mobile_jni::with_env(|env| {
            let activity = mobile_jni::activity(env)?;
            let service_name = env
                .new_string("clipboard")
                .map_err(|error| error.to_string())?;
            let service_name = JObject::from(service_name);
            let clipboard = env
                .call_method(
                    &activity,
                    jni::jni_str!("getSystemService"),
                    jni::jni_sig!("(Ljava/lang/String;)Ljava/lang/Object;"),
                    &[JValue::Object(&service_name)],
                )
                .and_then(|value: ::jni::objects::JValueOwned| value.l())
                .map_err(|error| error.to_string())?;
            let label = env
                .new_string("Pass Gen 1.0")
                .map_err(|error| error.to_string())?;
            let value = env.new_string(text).map_err(|error| error.to_string())?;
            let label = JObject::from(label);
            let value = JObject::from(value);
            let clip_data = env
                .call_static_method(
                    jni::jni_str!("android/content/ClipData"),
                    jni::jni_str!("newPlainText"),
                    jni::jni_sig!("(Ljava/lang/CharSequence;Ljava/lang/CharSequence;)Landroid/content/ClipData;"),
                    &[JValue::Object(&label), JValue::Object(&value)],
                )
                .and_then(|result: ::jni::objects::JValueOwned| result.l())
                .map_err(|error| error.to_string())?;
            env.call_method(
                &clipboard,
                jni::jni_str!("setPrimaryClip"),
                jni::jni_sig!("(Landroid/content/ClipData;)V"),
                &[JValue::Object(&clip_data)],
            )
            .map_err(|error| error.to_string())?;
            Ok(())
        });
    }

    #[cfg(not(target_os = "android"))]
    {
        let _ = text;
        Err("clipboard integration is available on Android".into())
    }
}

fn clear_clipboard() -> Result<(), String> {
    copy_to_clipboard("")
}

#[cfg(target_os = "android")]
fn android_safe_top_inset() -> f32 {
    gpui_mobile::android::jni::platform()
        .and_then(|platform| platform.primary_window())
        .map(|window| window.safe_area_insets_logical().top)
        .unwrap_or(0.0)
}

#[cfg(not(target_os = "android"))]
fn android_safe_top_inset() -> f32 {
    0.0
}

#[cfg(target_os = "android")]
pub fn open_main_window(cx: &mut gpui::App) {
    if let Err(error) = cx.text_system().add_fonts(vec![
        Cow::Borrowed(include_bytes!("../FSEX302.ttf") as &[u8]),
        Cow::Borrowed(include_bytes!("../jh_fallout-webfont.ttf") as &[u8]),
    ]) {
        log::warn!("Failed to load bundled Fallout fonts: {error}");
    }

    let _ = cx.open_window(
        gpui::WindowOptions {
            window_bounds: None,
            ..Default::default()
        },
        |_, cx| cx.new(|_| PasswordApp::new()),
    );
    cx.activate(true);
}
