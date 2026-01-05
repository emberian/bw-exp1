//! Footer component for landing pages

use leptos::prelude::*;

#[component]
pub fn Footer() -> impl IntoView {
    view! {
        <footer class="bg-slate-950 border-t border-slate-800">
            <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12">
                <div class="grid grid-cols-1 md:grid-cols-4 gap-8">
                    // Brand
                    <div class="col-span-1 md:col-span-2">
                        <span class="text-2xl font-bold text-amber-500">"BLACKWING"</span>
                        <p class="mt-4 text-slate-400 max-w-md">
                            "A text-based space opera where you command a starship in a universe where humanity has vanished. "
                            "Patrol the stars, uncover mysteries, and shape the future."
                        </p>
                    </div>

                    // Links
                    <div>
                        <h4 class="text-sm font-semibold text-slate-200 uppercase tracking-wider mb-4">"Game"</h4>
                        <ul class="space-y-2">
                            <li><a href="/play" class="text-slate-400 hover:text-white transition-colors">"Play Now"</a></li>
                            <li><a href="/login" class="text-slate-400 hover:text-white transition-colors">"Sign In"</a></li>
                            <li><a href="/register" class="text-slate-400 hover:text-white transition-colors">"Create Account"</a></li>
                        </ul>
                    </div>

                    // More links
                    <div>
                        <h4 class="text-sm font-semibold text-slate-200 uppercase tracking-wider mb-4">"About"</h4>
                        <ul class="space-y-2">
                            <li><a href="#story" class="text-slate-400 hover:text-white transition-colors">"Story"</a></li>
                            <li><a href="#features" class="text-slate-400 hover:text-white transition-colors">"Features"</a></li>
                        </ul>
                    </div>
                </div>

                <div class="mt-12 pt-8 border-t border-slate-800 text-center text-slate-500 text-sm">
                    <p>"BLACKWING - A Space Guard Chronicle"</p>
                </div>
            </div>
        </footer>
    }
}
