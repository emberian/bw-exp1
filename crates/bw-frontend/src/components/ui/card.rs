//! Card component

use leptos::prelude::*;

#[component]
pub fn Card(
    #[prop(optional)] class: &'static str,
    children: Children,
) -> impl IntoView {
    view! {
        <div class=format!("rounded-lg border border-slate-700 bg-slate-800 text-slate-100 shadow-sm {}", class)>
            {children()}
        </div>
    }
}

#[component]
pub fn CardHeader(
    #[prop(optional)] class: &'static str,
    children: Children,
) -> impl IntoView {
    view! {
        <div class=format!("flex flex-col space-y-1.5 p-4 {}", class)>
            {children()}
        </div>
    }
}

#[component]
pub fn CardTitle(
    #[prop(optional)] class: &'static str,
    children: Children,
) -> impl IntoView {
    view! {
        <h3 class=format!("text-lg font-semibold leading-none tracking-tight {}", class)>
            {children()}
        </h3>
    }
}

#[component]
pub fn CardContent(
    #[prop(optional)] class: &'static str,
    children: Children,
) -> impl IntoView {
    view! {
        <div class=format!("p-4 pt-0 {}", class)>
            {children()}
        </div>
    }
}
