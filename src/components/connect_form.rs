//! Credentials entry for an Xtream account.

use cathode_core::sources::xtream::XtreamCredentials;
use dioxus::prelude::*;

use crate::ui::button::{Button, ButtonVariant};

const INPUT: &str = "w-full rounded-md border border-neutral-300 dark:border-neutral-700 \
    bg-white dark:bg-neutral-900 px-3 py-2 text-sm outline-none \
    focus:ring-2 focus:ring-sky-500";

#[component]
pub fn ConnectForm(connecting: bool, on_connect: EventHandler<XtreamCredentials>) -> Element {
    let mut base_url = use_signal(String::new);
    let mut username = use_signal(String::new);
    let mut password = use_signal(String::new);

    let submit = move |event: FormEvent| {
        event.prevent_default();
        let creds = XtreamCredentials {
            base_url: base_url.read().trim().to_string(),
            username: username.read().trim().to_string(),
            password: password.read().clone(),
        };
        if creds.base_url.is_empty() || creds.username.is_empty() {
            return;
        }
        on_connect.call(creds);
    };

    rsx! {
        form {
            class: "flex flex-col gap-2 sm:flex-row sm:items-center",
            onsubmit: submit,
            input {
                class: INPUT,
                r#type: "text",
                placeholder: "http://host:port",
                value: "{base_url}",
                oninput: move |e| base_url.set(e.value()),
            }
            input {
                class: INPUT,
                r#type: "text",
                placeholder: "Username",
                value: "{username}",
                oninput: move |e| username.set(e.value()),
            }
            input {
                class: INPUT,
                r#type: "password",
                placeholder: "Password",
                value: "{password}",
                oninput: move |e| password.set(e.value()),
            }
            Button {
                variant: ButtonVariant::Primary,
                r#type: "submit",
                disabled: connecting,
                class: "shrink-0",
                if connecting { "Connecting…" } else { "Connect" }
            }
        }
    }
}
