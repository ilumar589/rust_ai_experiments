mod api;
mod components;
mod models;
mod state;
mod ws;

use leptos::prelude::*;
use leptos::mount::mount_to_body;

use components::chat::ChatArea;
use components::sidebar::Sidebar;
use state::AppState;

/// Root application component.
#[component]
fn App() -> impl IntoView {
    let state = AppState::provide();

    // Load conversations on mount
    state.load_conversations();

    view! {
        <div class="app-container">
            <Sidebar />
            <ChatArea />
        </div>
    }
}

fn main() {
    console_log::init_with_level(log::Level::Debug).expect("Failed to init logger");
    mount_to_body(App);
}
