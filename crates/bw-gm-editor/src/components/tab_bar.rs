//! Tab bar component

use leptos::prelude::*;

/// Available tabs in the GM Editor
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tab {
    Scripts,
    Config,
    Entities,
    Staged,
}

impl Tab {
    pub fn label(&self) -> &'static str {
        match self {
            Tab::Scripts => "Scripts",
            Tab::Config => "Config",
            Tab::Entities => "Entities",
            Tab::Staged => "Staged",
        }
    }

    pub fn all() -> &'static [Tab] {
        &[Tab::Scripts, Tab::Config, Tab::Entities, Tab::Staged]
    }
}

/// Tab bar component
#[component]
pub fn TabBar(active_tab: RwSignal<Tab>) -> impl IntoView {
    view! {
        <div class="flex border-b border-slate-700 bg-slate-800/30">
            {Tab::all().iter().map(|&tab| {
                let is_active = move || active_tab.get() == tab;
                view! {
                    <button
                        class="px-4 py-2 text-sm font-medium transition-colors"
                        class:text-amber-500=is_active
                        class:border-b-2=is_active
                        class:border-amber-500=is_active
                        class:text-slate-400=move || !is_active()
                        class:hover:text-slate-200=move || !is_active()
                        on:click=move |_| active_tab.set(tab)
                    >
                        {tab.label()}
                    </button>
                }
            }).collect_view()}
        </div>
    }
}
