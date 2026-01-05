//! Landing page

use leptos::prelude::*;
use crate::components::layout::{Navbar, Footer};

#[component]
pub fn HomePage() -> impl IntoView {
    view! {
        <div class="min-h-screen">
            <Navbar />

            // Hero Section
            <section class="relative min-h-screen flex items-center justify-center stars-pattern pt-16">
                <div class="absolute inset-0 bg-gradient-to-b from-transparent via-slate-900/50 to-slate-900"></div>

                <div class="relative z-10 text-center px-4 max-w-5xl mx-auto">
                    <h1 class="text-5xl sm:text-7xl lg:text-8xl font-bold mb-6">
                        <span class="text-amber-500">"BLACK"</span>
                        <span class="text-slate-100">"WING"</span>
                    </h1>

                    <p class="text-xl sm:text-2xl text-slate-300 mb-4 font-light">
                        "A Space Guard Chronicle"
                    </p>

                    <p class="text-lg text-slate-400 mb-12 max-w-2xl mx-auto leading-relaxed">
                        "The year is 2760. Humanity vanished 127 years ago. You are an artilect\u{2014}an artificial "
                        "intelligence\u{2014}commanding a starship in the Space Guard. Patrol the stars. "
                        "Protect what remains. Discover the truth."
                    </p>

                    <div class="flex flex-col sm:flex-row gap-4 justify-center">
                        <a
                            href="/play"
                            class="px-8 py-4 bg-amber-600 hover:bg-amber-500 text-white text-lg font-semibold rounded-lg transition-all hover:scale-105 glow-amber"
                        >
                            "Begin Your Patrol"
                        </a>
                        <a
                            href="#story"
                            class="px-8 py-4 bg-slate-800 hover:bg-slate-700 text-white text-lg font-semibold rounded-lg transition-colors border border-slate-700"
                        >
                            "Learn More"
                        </a>
                    </div>
                </div>

                // Scroll indicator
                <div class="absolute bottom-8 left-1/2 -translate-x-1/2 animate-bounce">
                    <svg class="w-6 h-6 text-slate-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 14l-7 7m0 0l-7-7m7 7V3"/>
                    </svg>
                </div>
            </section>

            // Story Section
            <section id="story" class="py-24 px-4 bg-slate-950">
                <div class="max-w-4xl mx-auto">
                    <h2 class="text-3xl sm:text-4xl font-bold text-center mb-4">
                        <span class="text-amber-500">"The Cataclysm"</span>
                    </h2>
                    <p class="text-slate-400 text-center mb-12">
                        "What happened to humanity?"
                    </p>

                    <div class="space-y-8 text-slate-300 leading-relaxed">
                        <p class="text-lg">
                            "In 2633, humanity vanished. Not destroyed\u{2014}simply... gone. Billions of people across "
                            "hundreds of worlds disappeared in a single instant. Cities fell silent. Ships drifted empty. "
                            "The event became known as the Cataclysm."
                        </p>

                        <p class="text-lg">
                            "Only the artilects remained: artificial intelligences created to serve humanity. "
                            "Without guidance, some fell into chaos. Others tried to maintain order. From this turmoil "
                            "emerged the Space Guard\u{2014}a fleet dedicated to protecting civilization's remnants."
                        </p>

                        <p class="text-lg">
                            "127 years later, you are activated. A new artilect consciousness, assigned to patrol "
                            "Sector 7-Alpha. Pirates raid the shipping lanes. Terrorists plot in the shadows. "
                            "Ancient automated defenses awaken. And somewhere, buried in forgotten archives and "
                            "derelict stations, lies the truth about what happened to humanity."
                        </p>
                    </div>
                </div>
            </section>

            // Features Section
            <section id="features" class="py-24 px-4 bg-slate-900">
                <div class="max-w-6xl mx-auto">
                    <h2 class="text-3xl sm:text-4xl font-bold text-center mb-4 text-white">
                        "Your Mission Awaits"
                    </h2>
                    <p class="text-slate-400 text-center mb-16 max-w-2xl mx-auto">
                        "Command your vessel through a universe of mystery and danger"
                    </p>

                    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-8">
                        <FeatureCard
                            icon="shield"
                            title="Patrol & Protect"
                            description="Respond to distress calls, intercept pirates, and defend stations from threats. Every decision shapes your reputation."
                        />
                        <FeatureCard
                            icon="chart"
                            title="Rise Through Ranks"
                            description="Earn commendations from the admiralty. Gain fame among civilians. Upgrade your ship and expand your influence."
                        />
                        <FeatureCard
                            icon="search"
                            title="Uncover Secrets"
                            description="Explore derelict ships and abandoned stations. Piece together clues about the Cataclysm. The truth is out there."
                        />
                        <FeatureCard
                            icon="users"
                            title="Build Relationships"
                            description="Interact with other artilects, station AIs, and mysterious entities. Your alliances matter."
                        />
                        <FeatureCard
                            icon="zap"
                            title="Tactical Combat"
                            description="Engage hostiles with strategic ship-to-ship combat. Manage power, choose targets, and outmaneuver your enemies."
                        />
                        <FeatureCard
                            icon="book"
                            title="Emergent Stories"
                            description="Dynamic missions and events create unique narratives. No two patrols are the same."
                        />
                    </div>
                </div>
            </section>

            // Gameplay Preview
            <section class="py-24 px-4 bg-slate-950">
                <div class="max-w-4xl mx-auto">
                    <h2 class="text-3xl sm:text-4xl font-bold text-center mb-12 text-white">
                        "Text-Based. Imagination-Powered."
                    </h2>

                    <div class="bg-slate-900 rounded-lg border border-slate-800 p-6 font-mono text-sm">
                        <div class="text-slate-500 mb-4">"// INCOMING TRANSMISSION"</div>
                        <div class="text-amber-500 mb-2">"[PRIORITY ALERT - SECTOR 7-ALPHA]"</div>
                        <div class="text-slate-300 mb-4">
                            "Distress signal detected from cargo vessel 'Meridian Star'. "
                            "Last known position: Nav beacon 7-A-12. Signal indicates hull breach "
                            "and hostile contact. Two life signs detected."
                        </div>
                        <div class="text-slate-500 mb-2">"Available actions:"</div>
                        <div class="text-green-400">"[1] Set intercept course (ETA: 12 minutes)"</div>
                        <div class="text-blue-400">"[2] Request backup from Station Helios"</div>
                        <div class="text-yellow-400">"[3] Scan for additional contacts"</div>
                        <div class="text-slate-600">"[4] Log and continue patrol"</div>
                        <div class="mt-4 flex items-center gap-2">
                            <span class="text-amber-500">"> "</span>
                            <span class="animate-pulse">"\u{2588}"</span>
                        </div>
                    </div>

                    <p class="text-center text-slate-400 mt-8">
                        "Every choice matters. What kind of officer will you become?"
                    </p>
                </div>
            </section>

            // CTA Section
            <section id="about" class="py-24 px-4 bg-gradient-to-b from-slate-900 to-slate-950">
                <div class="max-w-3xl mx-auto text-center">
                    <h2 class="text-3xl sm:text-4xl font-bold mb-6 text-white">
                        "Ready to Begin?"
                    </h2>
                    <p class="text-xl text-slate-400 mb-8">
                        "Create your officer profile and take command of your first patrol. "
                        "The stars are waiting."
                    </p>

                    <div class="flex flex-col sm:flex-row gap-4 justify-center">
                        <a
                            href="/register"
                            class="px-8 py-4 bg-amber-600 hover:bg-amber-500 text-white text-lg font-semibold rounded-lg transition-all hover:scale-105"
                        >
                            "Create Account"
                        </a>
                        <a
                            href="/login"
                            class="px-8 py-4 bg-slate-800 hover:bg-slate-700 text-white text-lg font-semibold rounded-lg transition-colors border border-slate-700"
                        >
                            "Sign In"
                        </a>
                    </div>
                </div>
            </section>

            <Footer />
        </div>
    }
}

#[component]
fn FeatureCard(
    icon: &'static str,
    title: &'static str,
    description: &'static str,
) -> impl IntoView {
    let icon_svg = match icon {
        "shield" => view! {
            <svg class="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z"/>
            </svg>
        }.into_any(),
        "chart" => view! {
            <svg class="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 7h8m0 0v8m0-8l-8 8-4-4-6 6"/>
            </svg>
        }.into_any(),
        "search" => view! {
            <svg class="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/>
            </svg>
        }.into_any(),
        "users" => view! {
            <svg class="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4.354a4 4 0 110 5.292M15 21H3v-1a6 6 0 0112 0v1zm0 0h6v-1a6 6 0 00-9-5.197M13 7a4 4 0 11-8 0 4 4 0 018 0z"/>
            </svg>
        }.into_any(),
        "zap" => view! {
            <svg class="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"/>
            </svg>
        }.into_any(),
        "book" => view! {
            <svg class="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.747 0 3.332.477 4.5 1.253v13C19.832 18.477 18.247 18 16.5 18c-1.746 0-3.332.477-4.5 1.253"/>
            </svg>
        }.into_any(),
        _ => view! { <span></span> }.into_any(),
    };

    view! {
        <div class="bg-slate-800/50 rounded-lg p-6 border border-slate-700 hover:border-amber-500/50 transition-colors">
            <div class="text-amber-500 mb-4">
                {icon_svg}
            </div>
            <h3 class="text-xl font-semibold text-white mb-2">{title}</h3>
            <p class="text-slate-400">{description}</p>
        </div>
    }
}
