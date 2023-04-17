use gloo_console::log;
use yew::prelude::*;
use yewtube::component::Player;

#[function_component(App)]
fn app() -> Html {
    let player_state = use_state(|| yewtube::ext::PlayerState::Unstarted);

    let update_player_state = {
        let player_state = player_state.clone();

        Callback::from(move |s: yewtube::ext::PlayerState| {
            log!("updating player state", format!("{:?}", s));
            player_state.set(s);
        })
    };

    html! {
        <div>
            <div>{format!("Player state: {:?}", *player_state)}</div>
            <Player video_id={"r71Nhzh0xMU"} on_state_change={update_player_state}/>
        </div>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
