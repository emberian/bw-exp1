//! Debug panel for performance metrics visualization

use leptos::prelude::*;

use crate::api::WsService;
use crate::state::{GameState, PhaseTimingInfo, SectorTimingInfo, TickMetricsHistoryInfo, TickMetricsInfo};

/// Debug panel showing tick performance metrics.
#[component]
pub fn DebugPanel() -> impl IntoView {
    let game_state = expect_context::<GameState>();
    let ws = expect_context::<WsService>();

    // Subscribe/unsubscribe based on panel visibility
    Effect::new(move |_| {
        if game_state.show_debug_panel.get() {
            ws.subscribe_metrics();
        } else {
            ws.unsubscribe_metrics();
        }
    });

    let is_visible = move || game_state.show_debug_panel.get();
    let metrics = move || game_state.tick_metrics.get();
    let history = move || game_state.tick_metrics_history.get();

    view! {
        <Show when=is_visible>
            <div class="fixed bottom-20 right-4 w-80 bg-slate-900/95 border border-slate-700 rounded-lg shadow-xl z-40 max-h-[70vh] overflow-hidden flex flex-col text-xs">
                // Header
                <div class="flex items-center justify-between px-3 py-2 border-b border-slate-700 bg-slate-800/50">
                    <h3 class="font-semibold text-amber-400">"Tick Metrics"</h3>
                    <button
                        class="text-slate-400 hover:text-slate-200"
                        on:click=move |_| game_state.show_debug_panel.set(false)
                    >
                        "×"
                    </button>
                </div>

                // Content
                <div class="flex-1 overflow-y-auto p-3 space-y-3">
                    // Current tick overview
                    {move || metrics().map(|m| view! { <TickOverview metrics=m /> })}

                    // Phase breakdown
                    {move || metrics().map(|m| view! { <PhaseBreakdown phases=m.phases total_us=m.total_us /> })}

                    // Sector breakdown
                    {move || metrics().map(|m| view! { <SectorBreakdown sectors=m.sectors /> })}

                    // History stats
                    {move || history().map(|h| view! { <HistoryStats history=h /> })}
                </div>
            </div>
        </Show>
    }
}

#[component]
fn TickOverview(metrics: TickMetricsInfo) -> impl IntoView {
    let budget_percent = (metrics.total_us as f64 / metrics.budget_us as f64 * 100.0).min(150.0);
    let bar_color = if metrics.over_budget {
        "bg-red-500"
    } else if budget_percent > 80.0 {
        "bg-amber-500"
    } else {
        "bg-green-500"
    };

    view! {
        <div class="space-y-1">
            <div class="flex justify-between text-slate-400">
                <span>"Tick "{metrics.tick}</span>
                <span class=move || if metrics.over_budget { "text-red-400" } else { "text-slate-300" }>
                    {format!("{:.2}ms / {:.0}ms", metrics.total_us as f64 / 1000.0, metrics.budget_us as f64 / 1000.0)}
                </span>
            </div>
            <div class="h-1.5 bg-slate-700 rounded overflow-hidden">
                <div
                    class=format!("h-full transition-all {}", bar_color)
                    style=format!("width: {}%", budget_percent.min(100.0))
                />
            </div>
        </div>
    }
}

#[component]
fn PhaseBreakdown(phases: Vec<PhaseTimingInfo>, total_us: u64) -> impl IntoView {
    view! {
        <div class="space-y-1">
            <h4 class="text-slate-500 font-medium">"Phases"</h4>
            {phases.into_iter().map(|phase| {
                let percent = if total_us > 0 && !phase.skipped {
                    phase.duration_us as f64 / total_us as f64 * 100.0
                } else {
                    0.0
                };
                let color = if phase.skipped { "bg-slate-600" } else { phase_color(&phase.name) };
                let name = phase.name.clone();
                let duration = if phase.skipped {
                    "skipped".to_string()
                } else {
                    format!("{:.0}us", phase.duration_us)
                };
                let pct = if phase.skipped {
                    "-".to_string()
                } else {
                    format!("{:.1}%", percent)
                };
                let text_class = if phase.skipped { "text-slate-600" } else { "text-slate-300" };
                view! {
                    <div class="flex items-center gap-2">
                        <div class=format!("w-1.5 h-1.5 rounded-full {}", color) />
                        <span class=format!("flex-1 {}", text_class)>{name}</span>
                        <span class="text-slate-500 tabular-nums">{duration}</span>
                        <span class="text-slate-600 w-10 text-right tabular-nums">{pct}</span>
                    </div>
                }
            }).collect_view()}
        </div>
    }
}

#[component]
fn SectorBreakdown(sectors: Vec<SectorTimingInfo>) -> impl IntoView {
    let show_expanded = RwSignal::new(false);
    let sector_count = sectors.len();

    // Pre-compute all the sector views eagerly
    let sector_content: Vec<_> = sectors.into_iter().map(|sector| {
        let name = sector.sector_name.clone();
        let total = format!("{:.0}us", sector.total_us);
        let sector_total_us = sector.total_us;
        // Only show non-skipped phases in the bar chart
        let phase_bars: Vec<_> = sector.phases.iter()
            .filter(|p| !p.skipped)
            .map(|p| {
                let width = if sector_total_us > 0 {
                    (p.duration_us as f64 / sector_total_us as f64 * 100.0).max(3.0)
                } else { 0.0 };
                let color = phase_color(&p.name);
                let style = format!("width: {}%", width);
                let title = format!("{}: {}us", p.name, p.duration_us);
                let class = format!("h-1 rounded {}", color);
                view! { <div class=class style=style title=title /> }
            }).collect();
        view! {
            <div>
                <div class="flex justify-between text-slate-400">
                    <span>{name}</span>
                    <span class="text-slate-500 tabular-nums">{total}</span>
                </div>
                <div class="flex gap-0.5 mt-0.5">
                    {phase_bars}
                </div>
            </div>
        }
    }).collect();

    view! {
        <div class="space-y-1">
            <button
                class="flex items-center gap-1 text-slate-500 font-medium hover:text-slate-400"
                on:click=move |_| show_expanded.update(|v| *v = !*v)
            >
                <span class=move || if show_expanded.get() { "rotate-90" } else { "" }>"▶"</span>
                "Sectors ("{sector_count}")"
            </button>
            <div
                class="space-y-2 pl-2 border-l border-slate-700"
                style=move || if show_expanded.get() { "display: block" } else { "display: none" }
            >
                {sector_content}
            </div>
        </div>
    }
}

#[component]
fn HistoryStats(history: TickMetricsHistoryInfo) -> impl IntoView {
    view! {
        <div class="grid grid-cols-4 gap-1 text-center">
            <div class="bg-slate-800/50 rounded px-1 py-0.5">
                <div class="text-slate-500">"Avg"</div>
                <div class="text-slate-300 tabular-nums">{format!("{:.1}ms", history.avg_duration_us as f64 / 1000.0)}</div>
            </div>
            <div class="bg-slate-800/50 rounded px-1 py-0.5">
                <div class="text-slate-500">"P95"</div>
                <div class="text-slate-300 tabular-nums">{format!("{:.1}ms", history.p95_duration_us as f64 / 1000.0)}</div>
            </div>
            <div class="bg-slate-800/50 rounded px-1 py-0.5">
                <div class="text-slate-500">"Max"</div>
                <div class="text-slate-300 tabular-nums">{format!("{:.1}ms", history.max_duration_us as f64 / 1000.0)}</div>
            </div>
            <div class="bg-slate-800/50 rounded px-1 py-0.5">
                <div class="text-slate-500">"Over"</div>
                <div class={if history.over_budget_count > 0 { "text-red-400 tabular-nums" } else { "text-green-400 tabular-nums" }}>
                    {history.over_budget_count}
                </div>
            </div>
        </div>
    }
}

/// Get color class for a phase name.
fn phase_color(name: &str) -> &'static str {
    match name {
        "coroutines" => "bg-purple-500",
        "behaviors" => "bg-blue-500",
        "sectors_total" => "bg-green-500",
        "global" => "bg-amber-500",
        "movement" => "bg-cyan-400",
        "combat" => "bg-red-400",
        "npc_spawn" => "bg-emerald-400",
        "npc_cleanup" => "bg-teal-400",
        "mission_spawn" => "bg-indigo-400",
        "mission_expire" => "bg-violet-400",
        "broadcast" => "bg-pink-400",
        _ => "bg-slate-500",
    }
}
