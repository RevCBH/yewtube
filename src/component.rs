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
    Initialized {
        ext_player: Rc<ext::Player>,
        on_ready: Rc<Closure<dyn FnMut(JsValue)>>,
    },
    Ready {
        ext_player: Rc<ext::Player>,
        on_state_change: Rc<Closure<dyn FnMut(JsValue)>>,
    },
    Failed,
}

trait FsmState {
    fn transition(self, msg: Msg, ctx: &Context<Player>) -> PlayerState;
}

#[derive(Clone)]
struct Uninitialized;

impl FsmState for Uninitialized {
    fn transition(self, msg: Msg, ctx: &Context<Player>) -> PlayerState {
        match msg {
            Msg::Initialized => {
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

                PlayerState::Initialized(Initialized {
                    ext_player: Rc::new(ext_player),
                    on_ready: closure_ready,
                })
            }
            _ => PlayerState::Failed(Failed {
                err: format!("Invalid message {:?} in Uninitialized state", msg),
                _stale_closures: vec![],
            }),
        }
    }
}

#[derive(Clone)]
struct Initialized {
    ext_player: Rc<ext::Player>,
    on_ready: Rc<Closure<dyn FnMut(JsValue)>>,
}

impl FsmState for Initialized {
    fn transition(self, msg: Msg, ctx: &Context<Player>) -> PlayerState {
        match msg {
            Msg::Ready => {
                let callback_state_change = ctx.link().callback(|x| match x {
                    Ok(new_state) => Msg::PlayerStateChange(new_state),
                    Err(e) => Msg::Failed(e),
                });

                let closure_state_change = Rc::new(Closure::new(move |s: JsValue| {
                    callback_state_change.emit(s.try_into());
                }));

                self.ext_player
                    .add_event_listener("onStateChange", closure_state_change.as_ref());
                self.ext_player
                    .remove_event_listener("onReady", self.on_ready.as_ref());

                PlayerState::Ready(Ready {
                    ext_player: self.ext_player,
                    on_state_change: closure_state_change,
                })
            }
            _ => PlayerState::Failed(Failed {
                err: format!("Invalid message {:?} in Initialized state", msg),
                _stale_closures: vec![self.on_ready],
            }),
        }
    }
}

#[derive(Clone)]
struct Ready {
    ext_player: Rc<ext::Player>,
    on_state_change: Rc<Closure<dyn FnMut(JsValue)>>,
}

impl FsmState for Ready {
    fn transition(self, msg: Msg, ctx: &Context<Player>) -> PlayerState {
        match msg {
            Msg::PlayerStateChange(s) => {
                if let Some(cb) = &ctx.props().on_state_change {
                    cb.emit(s);
                }
                PlayerState::Ready(self)
            }
            _ => {
                // ISSUE: this doesn't work. I think event listeners need to be registered via name to be removed.
                self.ext_player
                    .remove_event_listener("onStateChange", self.on_state_change.as_ref());
                PlayerState::Failed(Failed {
                    err: format!("Invalid message {:?} in Ready state", msg),
                    _stale_closures: vec![self.on_state_change],
                })
            }
        }
    }
}

#[derive(Clone)]
struct Failed {
    err: String,
    // TODO: remove this once we have a way to remove event listeners
    _stale_closures: Vec<Rc<Closure<dyn FnMut(JsValue)>>>,
}

impl FsmState for Failed {
    fn transition(self, _msg: Msg, _ctx: &Context<Player>) -> PlayerState {
        PlayerState::Failed(self)
    }
}

#[derive(Clone)]
enum PlayerState {
    Uninitialized(Uninitialized),
    Initialized(Initialized),
    Ready(Ready),
    Failed(Failed),
}

impl FsmState for PlayerState {
    fn transition(self, msg: Msg, ctx: &Context<Player>) -> PlayerState {
        match self {
            PlayerState::Uninitialized(s) => s.transition(msg, ctx),
            PlayerState::Initialized(s) => s.transition(msg, ctx),
            PlayerState::Ready(s) => s.transition(msg, ctx),
            PlayerState::Failed(s) => s.transition(msg, ctx),
        }
    }
}

pub struct Player {
    state: PlayerState,
}

#[derive(Debug)]
pub enum Msg {
    Initialized,
    Ready,
    PlayerStateChange(ext::PlayerState),
    Failed(JsValue),
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
            state: PlayerState::Uninitialized(Uninitialized),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        self.state = self.state.clone().transition(msg, ctx);

        // Only rerender if we're in a failed state
        match &self.state {
            PlayerState::Failed(_) => true,
            _ => false,
        }
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        match &self.state {
            PlayerState::Failed(err) => html! {
                // ISSUE: This span is necessary to actually unmount the iframe that replaces
                // the placeholder div. Without it, the yew virtual dom doesn't seem to realize
                // that the wrapper divs are different, despite having different IDs
                <span id="youtube-player-error-wrapper">
                    <div class="youtube-player-error">{err.err.clone()}</div>
                </span>
            },

            _ => html! {
                <div id="youtube-player-wrapper">
                    <div id="youtube-player-placeholder"></div>
                </div>
            },
        }
    }

    fn rendered(&mut self, _ctx: &Context<Self>, _first_render: bool) {}
}
