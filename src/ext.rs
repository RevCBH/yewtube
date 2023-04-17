use js_sys::Function;
use serde::Serialize;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlScriptElement;

pub async fn load_iframe_api() -> Result<JsValue, JsValue> {
    let window = web_sys::window().ok_or("No global `window` exists")?;
    let document = window
        .document()
        .ok_or("Should have a document on window")?;

    let script = document
        .create_element("script")?
        .dyn_into::<HtmlScriptElement>()?;
    script.set_src("https://www.youtube.com/iframe_api");

    let (sender, receiver) = futures::channel::oneshot::channel();

    let onload_callback = Closure::once(move || {
        let _ = sender.send(Ok(JsValue::TRUE));
    });
    script.set_onload(Some(onload_callback.as_ref().unchecked_ref()));
    onload_callback.forget();

    document.body().unwrap().append_child(&script)?;

    let result = receiver.await.unwrap();
    result
}

pub fn is_yt_ready() -> bool {
    let Some(window) = web_sys::window() else { return false };
    let Ok(yt) = js_sys::Reflect::get(&window, &"YT".into()) else {return false};
    let Ok(_) = js_sys::Reflect::has(&yt, &"Player".into()) else {return false};
    return true;
}

pub fn unsafe_get_player() -> Function {
    let window = web_sys::window().unwrap();
    let yt = js_sys::Reflect::get(&window, &"YT".into()).unwrap();
    let player = js_sys::Reflect::get(&yt, &"Player".into()).unwrap();
    player.try_into().unwrap()
}

pub async fn yt_iframe_api_ready() -> Result<JsValue, JsValue> {
    let window = web_sys::window().unwrap();
    let yt = js_sys::Reflect::get(&window, &"YT".into())?;

    if !yt.is_undefined() && js_sys::Reflect::has(&yt, &"Player".into())? {
        Ok(yt)
    } else {
        load_iframe_api().await?;
        let (sender, receiver) = futures::channel::oneshot::channel();
        let resolve_closure = Closure::once(move || {
            let _: Result<(), Result<JsValue, Option<JsValue>>> = sender.send(Ok(JsValue::TRUE));
        });

        js_sys::Reflect::set(
            &window,
            &"onYouTubeIframeAPIReady".into(),
            &resolve_closure.into_js_value(),
        )?;

        let _ = receiver
            .await
            .map_err(|e| format!("failed to await the youtube iframe api: {}", e.to_string()))?;

        Ok(js_sys::Reflect::get(&window, &"YT".into())?)
    }
}

#[derive(Serialize, Default)]
pub struct PlayerVars {
    #[serde(rename = "autoplay", skip_serializing_if = "Option::is_none")]
    pub autoplay: Option<u8>,
    #[serde(rename = "controls", skip_serializing_if = "Option::is_none")]
    pub controls: Option<u8>,
    #[serde(rename = "enablejsapi", skip_serializing_if = "Option::is_none")]
    pub enable_js_api: Option<u8>,
    #[serde(rename = "fs", skip_serializing_if = "Option::is_none")]
    pub full_screen: Option<u8>,
    #[serde(rename = "iv_load_policy", skip_serializing_if = "Option::is_none")]
    pub iv_load_policy: Option<u8>,
    #[serde(rename = "modestbranding", skip_serializing_if = "Option::is_none")]
    pub modest_branding: Option<u8>,
    #[serde(rename = "playsinline", skip_serializing_if = "Option::is_none")]
    pub plays_inline: Option<u8>,
    #[serde(rename = "rel", skip_serializing_if = "Option::is_none")]
    pub related_videos: Option<u8>,
    #[serde(rename = "showinfo", skip_serializing_if = "Option::is_none")]
    pub show_info: Option<u8>,
    #[serde(rename = "start", skip_serializing_if = "Option::is_none")]
    pub start: Option<u32>,
    #[serde(rename = "end", skip_serializing_if = "Option::is_none")]
    pub end: Option<u32>,
    #[serde(rename = "origin", skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
    #[serde(rename = "widget_referrer", skip_serializing_if = "Option::is_none")]
    pub widget_referrer: Option<String>,
}

#[derive(Serialize, Default)]
pub struct Options {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(rename = "videoId", skip_serializing_if = "Option::is_none")]
    pub video_id: Option<String>,
    #[serde(rename = "playerVars", skip_serializing_if = "Option::is_none")]
    pub player_vars: Option<PlayerVars>,
}

#[derive(Debug)]
pub enum PlayerState {
    Unstarted = -1,
    Ended = 0,
    Playing = 1,
    Paused = 2,
    Buffering = 3,
    Cued = 5,
}

impl TryFrom<JsValue> for PlayerState {
    type Error = JsValue;

    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        let state = js_sys::Reflect::get(&value, &"data".into())?
            .as_f64()
            .ok_or("invalid player state")?;
        match state as i32 {
            -1 => Ok(PlayerState::Unstarted),
            0 => Ok(PlayerState::Ended),
            1 => Ok(PlayerState::Playing),
            2 => Ok(PlayerState::Paused),
            3 => Ok(PlayerState::Buffering),
            5 => Ok(PlayerState::Cued),
            _ => Err("invalid player state".into()),
        }
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = YT, js_name=Player)]
    pub type Player;

    #[wasm_bindgen(js_namespace = YT, js_class="Player", constructor)]
    pub fn new(target_id: &str, options: JsValue) -> Player;

    #[wasm_bindgen(method, js_name=playVideo)]
    pub fn play_video(this: &Player);

    #[wasm_bindgen(method, js_name=cueVideoById)]
    pub fn cue_video_by_id(this: &Player, video_id: JsValue);

    #[wasm_bindgen(method, js_name=addEventListener)]
    pub fn add_event_listener(this: &Player, event: &str, callback: &Closure<dyn FnMut(JsValue)>);

    #[wasm_bindgen(method, js_name=removeEventListener)]
    pub fn remove_event_listener(
        this: &Player,
        event: &str,
        callback: &Closure<dyn FnMut(JsValue)>,
    );

    #[wasm_bindgen(method, js_name=getPlayerState)]
    fn _get_player_state(this: &Player) -> i32;
}

impl Player {
    pub fn get_player_state(&self) -> PlayerState {
        match self._get_player_state() {
            -1 => PlayerState::Unstarted,
            0 => PlayerState::Ended,
            1 => PlayerState::Playing,
            2 => PlayerState::Paused,
            3 => PlayerState::Buffering,
            5 => PlayerState::Cued,
            _ => panic!("unknown player state"),
        }
    }
}
