use std::rc::Rc;

use crate::ext;
use futures::FutureExt;
use wasm_bindgen::{prelude::Closure, JsValue};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

#[derive(Clone, Properties, PartialEq)]
pub struct Props {
    pub video_id: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub autoplay: Option<bool>,

    pub on_state_change: Option<Callback<ext::PlayerState>>,
}

pub enum State {
    Uninitialized,
    Initialized(Rc<ext::Player>, Rc<Closure<dyn FnMut(JsValue)>>),
    Ready {
        ext_player: Rc<ext::Player>,
        on_state_change: Rc<Closure<dyn FnMut(JsValue)>>,
    },
}

pub struct Player {
    state: State,
}

pub enum Msg {
    Initialized,
    Ready,
    PlayerStateChange(ext::PlayerState),
}

impl yew::Component for Player {
    type Message = Msg;
    type Properties = Props;

    fn create(_ctx: &Context<Self>) -> Self {
        let cb: Callback<()> = _ctx.link().callback(|_| Msg::Initialized);
        spawn_local(ext::yt_iframe_api_ready().map(move |_| {
            cb.emit(());
        }));

        Self {
            state: State::Uninitialized,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match (&self.state, msg) {
            (State::Uninitialized, Msg::Initialized) => {
                let ext_player = ext::Player::new(
                    "youtube-player-placeholder",
                    serde_wasm_bindgen::to_value(&ext::Options {
                        video_id: Some(ctx.props().video_id.clone()),
                        width: ctx.props().width.clone(),
                        height: ctx.props().height.clone(),
                        player_vars: Some(ext::PlayerVars {
                            autoplay: ctx.props().autoplay.map(|x| if x { 1 } else { 0 }),
                            ..Default::default()
                        }),
                    })
                    .unwrap(),
                );
                let callback_ready = ctx.link().callback(|_| Msg::Ready);
                let closure_ready = Rc::new(Closure::once(move |_: JsValue| {
                    callback_ready.emit(());
                }));
                ext_player.add_event_listener("onReady", closure_ready.as_ref());

                self.state = State::Initialized(Rc::new(ext_player), closure_ready);
            }
            (State::Initialized(ext_player, cb), Msg::Ready) => {
                ext_player.remove_event_listener("onReady", cb.as_ref());
                let ext_player_clone = ext_player.clone();
                let callback_state_change = ctx
                    .link()
                    .callback(move |_| Msg::PlayerStateChange(ext_player_clone.get_player_state()));
                let closure_state_change = Rc::new(Closure::new(move |_: JsValue| {
                    callback_state_change.emit(())
                }));

                ext_player.add_event_listener("onStateChange", closure_state_change.as_ref());

                self.state = State::Ready {
                    ext_player: ext_player.clone(),
                    on_state_change: closure_state_change,
                };
            }
            (
                State::Ready {
                    ext_player: _,
                    on_state_change: _,
                },
                Msg::PlayerStateChange(s),
            ) => {
                if let Some(cb) = &ctx.props().on_state_change {
                    cb.emit(s);
                }
            }
            _ => panic!("unreachable yewtube::Player state"),
        }
        false
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        html! {
            <div id="youtube-player-placeholder"></div>
        }
    }

    fn rendered(&mut self, _ctx: &Context<Self>, _first_render: bool) {}
}
