//! Button component

use leptos::prelude::*;

#[derive(Default, Clone, Copy)]
pub enum ButtonVariant {
    #[default]
    Default,
    Primary,
    Secondary,
    Destructive,
    Ghost,
}

#[derive(Default, Clone, Copy)]
pub enum ButtonSize {
    #[default]
    Default,
    Small,
    Large,
}

#[component]
pub fn Button(
    #[prop(optional)] variant: ButtonVariant,
    #[prop(optional)] size: ButtonSize,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] class: &'static str,
    children: Children,
) -> impl IntoView {
    let base_classes = "inline-flex items-center justify-center rounded-md font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50";

    let variant_classes = match variant {
        ButtonVariant::Default => "bg-slate-700 text-slate-100 hover:bg-slate-600",
        ButtonVariant::Primary => "bg-amber-600 text-white hover:bg-amber-500",
        ButtonVariant::Secondary => "bg-slate-600 text-slate-100 hover:bg-slate-500",
        ButtonVariant::Destructive => "bg-red-600 text-white hover:bg-red-500",
        ButtonVariant::Ghost => "hover:bg-slate-700 hover:text-slate-100",
    };

    let size_classes = match size {
        ButtonSize::Default => "h-10 px-4 py-2",
        ButtonSize::Small => "h-8 px-3 text-sm",
        ButtonSize::Large => "h-12 px-6 text-lg",
    };

    view! {
        <button
            disabled=disabled
            class=format!("{} {} {} {}", base_classes, variant_classes, size_classes, class)
        >
            {children()}
        </button>
    }
}
