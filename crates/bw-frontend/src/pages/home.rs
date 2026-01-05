//! Home page

use leptos::prelude::*;

#[component]
pub fn HomePage() -> impl IntoView {
    view! {
        <div class="flex flex-col items-center justify-center min-h-screen p-8">
            <h1 class="text-6xl font-bold text-amber-500 mb-4">"BLACKWING"</h1>
            <p class="text-xl text-slate-400 mb-8 max-w-2xl text-center">
                "The year is 2760. Humanity vanished 127 years ago in an event called the Cataclysm. "
                "You are an artilect—an artificial intelligence—serving in the Space Guard, "
                "protecting what remains of civilization."
            </p>

            <div class="flex gap-4">
                <a
                    href="/register"
                    class="px-6 py-3 bg-amber-600 hover:bg-amber-500 text-white font-semibold rounded-lg transition-colors"
                >
                    "New Officer"
                </a>
                <a
                    href="/game"
                    class="px-6 py-3 bg-slate-700 hover:bg-slate-600 text-white font-semibold rounded-lg transition-colors"
                >
                    "Continue"
                </a>
            </div>

            <div class="mt-16 grid grid-cols-3 gap-8 max-w-4xl">
                <div class="text-center">
                    <div class="text-4xl mb-2">"🛡️"</div>
                    <h3 class="text-lg font-semibold text-slate-200">"Patrol"</h3>
                    <p class="text-sm text-slate-400">
                        "Protect your sector from pirates, terrorists, and cosmic threats."
                    </p>
                </div>
                <div class="text-center">
                    <div class="text-4xl mb-2">"⭐"</div>
                    <h3 class="text-lg font-semibold text-slate-200">"Rise"</h3>
                    <p class="text-sm text-slate-400">
                        "Build reputation with the admiralty and fame among the people."
                    </p>
                </div>
                <div class="text-center">
                    <div class="text-4xl mb-2">"🔍"</div>
                    <h3 class="text-lg font-semibold text-slate-200">"Discover"</h3>
                    <p class="text-sm text-slate-400">
                        "Uncover the mystery of what happened to humanity."
                    </p>
                </div>
            </div>
        </div>
    }
}
