//! Navigation bar component for landing pages

use leptos::prelude::*;

#[component]
pub fn Navbar() -> impl IntoView {
    let (menu_open, set_menu_open) = signal(false);

    view! {
        <nav class="fixed top-0 left-0 right-0 z-50 bg-slate-900/80 backdrop-blur-md border-b border-slate-800">
            <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
                <div class="flex items-center justify-between h-16">
                    // Logo
                    <a href="/" class="flex items-center gap-2">
                        <span class="text-2xl font-bold text-amber-500">"BLACKWING"</span>
                    </a>

                    // Desktop nav
                    <div class="hidden md:flex items-center gap-8">
                        <a href="#features" class="text-slate-300 hover:text-white transition-colors">"Features"</a>
                        <a href="#story" class="text-slate-300 hover:text-white transition-colors">"Story"</a>
                        <a href="#about" class="text-slate-300 hover:text-white transition-colors">"About"</a>
                        <a
                            href="/play"
                            class="px-4 py-2 bg-amber-600 hover:bg-amber-500 text-white font-semibold rounded-lg transition-colors"
                        >
                            "Play Now"
                        </a>
                    </div>

                    // Mobile menu button
                    <button
                        class="md:hidden p-2 text-slate-300 hover:text-white"
                        on:click=move |_| set_menu_open.update(|v| *v = !*v)
                    >
                        <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h16"/>
                        </svg>
                    </button>
                </div>

                // Mobile menu
                <div class=move || if menu_open.get() { "md:hidden pb-4" } else { "hidden" }>
                    <div class="flex flex-col gap-4 pt-4">
                        <a href="#features" class="text-slate-300 hover:text-white transition-colors">"Features"</a>
                        <a href="#story" class="text-slate-300 hover:text-white transition-colors">"Story"</a>
                        <a href="#about" class="text-slate-300 hover:text-white transition-colors">"About"</a>
                        <a
                            href="/play"
                            class="px-4 py-2 bg-amber-600 hover:bg-amber-500 text-white font-semibold rounded-lg transition-colors text-center"
                        >
                            "Play Now"
                        </a>
                    </div>
                </div>
            </div>
        </nav>
    }
}
