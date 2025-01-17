#![allow(clippy::unused_unit)]
use async_recursion::async_recursion;
use backtrace::Backtrace;
use js_sys::Promise;
use serde::{Deserialize, Serialize};
use std::panic;
use wasm_bindgen_futures::future_to_promise;

use hakim_engine::{
    all_library_data,
    engine::Engine,
    interactive::{tactic::Error, Session, Suggestion},
    notation_list,
};
use wasm_bindgen::prelude::*;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
extern "C" {
    fn panic_handler(s: &str);
    async fn ask_question(s: &str) -> JsValue;
}

#[wasm_bindgen]
#[derive(Default, Serialize, Deserialize)]
pub struct Instance {
    session: Option<Session>,
}

#[wasm_bindgen(start)]
pub fn start() {
    use std::sync::Once;
    static SET_HOOK: Once = Once::new();
    SET_HOOK.call_once(|| {
        panic::set_hook(Box::new(|p| {
            panic_handler(&format!(
                "Panic on rust side. This is a bug. The page \
        will no longer work properly. Reload the page.\n\nMore data:\n{}\nBacktrace:\n{:?}",
                p,
                Backtrace::new(),
            ))
        }));
    });
}

#[derive(Serialize, Deserialize)]
enum Monitor {
    SessionIsNotStarted,
    Finished,
    Running {
        hyps: Vec<(String, String)>,
        goals: Vec<String>,
    },
}

#[async_recursion(?Send)]
async fn run_tactic_inner(session: &mut Session, tactic: &str) -> Option<String> {
    match session.run_tactic(tactic) {
        Ok(_) => None,
        Err(Error::CanNotFindInstance(e)) => {
            let mut qt = e.question_text();
            loop {
                if let Some(ans) = ask_question(&qt).await.as_string() {
                    if ans.trim() == "" {
                        return None;
                    } else {
                        let e = e.clone();
                        match e.tactic_by_answer(&ans) {
                            Ok(t) => match run_tactic_inner(session, &t).await {
                                None => return None,
                                Some(e) => {
                                    qt = format!("$error: {e:?}\n{qt}");
                                    continue;
                                }
                            },
                            Err(e) => {
                                qt = format!("$error: {e:?}\n{qt}");
                                continue;
                            }
                        }
                    }
                } else {
                    return Some("bad output of ask_question".to_string());
                }
            }
        }
        Err(e) => Some(format!("{:?}", e)),
    }
}

#[wasm_bindgen]
impl Instance {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        start();
        Instance { session: None }
    }

    #[wasm_bindgen]
    pub fn to_backup(&self) -> JsValue {
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        self.serialize(&serializer).unwrap()
    }

    #[wasm_bindgen]
    pub fn from_backup(&mut self, json: JsValue) -> bool {
        match serde_wasm_bindgen::from_value(json) {
            Ok(x) => {
                *self = x;
                true
            }
            Err(_) => false,
        }
    }

    #[wasm_bindgen]
    pub fn start_session(&mut self, goal: &str, libs: &str, params: &str) -> Option<String> {
        let mut eng = Engine::new(params);
        for lib in libs.split(',') {
            if let Err(e) = eng.load_library(lib) {
                return Some(format!("{:?}", e));
            }
        }
        self.session = match eng.interactive_session(goal) {
            Ok(s) => Some(s),
            Err(e) => return Some(format!("{:?}", e)),
        };
        None
    }

    #[wasm_bindgen]
    pub fn start_session_from_lib(
        &mut self,
        lib: &str,
        name: &str,
        review: bool,
    ) -> Option<String> {
        self.session = match Session::from_middle_of_lib(lib, name, review) {
            Some(s) => Some(s),
            None => return Some("invalid".to_string()),
        };
        None
    }

    #[wasm_bindgen]
    pub fn all_library_data(&self) -> JsValue {
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        all_library_data().serialize(&serializer).unwrap()
    }

    #[wasm_bindgen]
    pub fn monitor(&self) -> JsValue {
        let monitor = self.session.as_ref().map(|s| s.monitor(true));
        serde_wasm_bindgen::to_value(&monitor).unwrap()
    }

    #[wasm_bindgen]
    pub fn natural(&self) -> Option<String> {
        let s = (&self.session).as_ref()?;
        Some(s.natural())
    }

    pub fn try_auto_history(&self) -> JsValue {
        if let Some(x) = &self.session {
            serde_wasm_bindgen::to_value(&x.history_based_auto()).unwrap()
        } else {
            JsValue::UNDEFINED
        }
    }

    pub fn try_auto(&self) -> Option<String> {
        let s = (&self.session).as_ref()?.clone();
        s.try_auto()
    }

    pub fn try_tactic(&self, tactic: &str) -> bool {
        let session = match &self.session {
            Some(s) => s,
            None => return false,
        };
        match session.clone().run_tactic(tactic) {
            Ok(_) => true,
            Err(e) => e.is_actionable(),
        }
    }

    #[wasm_bindgen]
    pub fn run_tactic(&mut self, tactic: String) -> Promise {
        let this = unsafe { std::mem::transmute::<&mut Instance, &'static mut Instance>(self) };
        future_to_promise(async move {
            let session = match &mut this.session {
                Some(s) => s,
                None => return Ok("session not started".into()),
            };
            let r = run_tactic_inner(session, &tactic).await;
            Ok(r.into())
        })
    }

    #[wasm_bindgen]
    pub fn get_history(&self) -> JsValue {
        let session = match &self.session {
            Some(s) => s,
            None => return JsValue::UNDEFINED,
        };
        serde_wasm_bindgen::to_value(&session.get_history()).unwrap()
    }

    async fn run_sugg(&mut self, sugg: Suggestion) -> Option<String> {
        let session = match &mut self.session {
            Some(s) => s,
            None => return Some("Session is not started".to_string()),
        };
        let mut v = vec![];
        for x in &sugg.questions {
            let x = match ask_question(x).await.as_string() {
                Some(x) => x,
                None => return Some("invalid question type".to_string()),
            };
            if x.trim() == "" {
                return None;
            }
            v.push(x);
        }
        match session.run_suggestion(sugg, v) {
            Ok(_) => None,
            Err(e) => Some(format!("{:?}", e)),
        }
    }

    pub fn suggest_dblclk_goal(&mut self) -> Promise {
        let this = unsafe { std::mem::transmute::<&mut Instance, &'static mut Instance>(self) };
        future_to_promise(async move {
            let session = match &mut this.session {
                Some(s) => s,
                None => return Ok("Session is not started".into()),
            };
            let sugg = match session.suggest_on_goal_dblclk() {
                Some(s) => s,
                None => return Ok("No suggestion for this type of goal".into()),
            };
            Ok(this.run_sugg(sugg).await.into())
        })
    }

    pub fn suggest_dblclk_hyp(&mut self, hyp_name: String) -> Promise {
        let this = unsafe { std::mem::transmute::<&mut Instance, &'static mut Instance>(self) };
        future_to_promise(async move {
            let session = match &mut this.session {
                Some(s) => s,
                None => return Ok("Session is not started".into()),
            };
            let sugg = match session.suggest_on_hyp_dblclk(&hyp_name) {
                Some(s) => s,
                None => return Ok("No suggestion for this type of hyp".into()),
            };
            Ok(this.run_sugg(sugg).await.into())
        })
    }

    pub fn suggest_menu_goal(&mut self) -> Option<String> {
        let session = &mut self.session.as_ref()?;
        let sugg = session.suggest_on_goal_menu();
        Some(
            sugg.into_iter()
                .map(|x| {
                    if x.is_default() {
                        format!("(★{})", x.class)
                    } else {
                        format!("({})", x.class)
                    }
                })
                .collect(),
        )
    }

    pub fn suggest_menu_hyp(&mut self, hyp_name: &str) -> Option<String> {
        let session = &mut self.session.as_ref()?;
        let sugg = session.suggest_on_hyp_menu(hyp_name);
        Some(
            sugg.into_iter()
                .map(|x| {
                    if x.is_default() {
                        format!("(★{})", x.class)
                    } else {
                        format!("({})", x.class)
                    }
                })
                .collect(),
        )
    }

    pub fn run_suggest_menu_hyp(&mut self, hyp_name: String, i: usize) -> Promise {
        let this = unsafe { std::mem::transmute::<&mut Instance, &'static mut Instance>(self) };
        future_to_promise(async move {
            let session = match &mut this.session {
                Some(s) => s,
                None => return Ok("Session is not started".into()),
            };
            let sugg = session.suggest_on_hyp_menu(&hyp_name);
            Ok(this
                .run_sugg(
                    sugg.into_iter()
                        .nth(i)
                        .ok_or("Bug in run_suggest_menu_hyp")?,
                )
                .await
                .into())
        })
    }

    pub fn run_suggest_menu_goal(&mut self, i: usize) -> Promise {
        let this = unsafe { std::mem::transmute::<&mut Instance, &'static mut Instance>(self) };
        future_to_promise(async move {
            let session = match &mut this.session {
                Some(s) => s,
                None => return Ok("Session is not started".into()),
            };
            let sugg = session.suggest_on_goal_menu();
            Ok(this
                .run_sugg(
                    sugg.into_iter()
                        .nth(i)
                        .ok_or("Bug in run_suggest_menu_goal")?,
                )
                .await
                .into())
        })
    }

    pub fn pos_of_span_hyp(&self, hyp: &str, l: usize, r: usize) -> Option<usize> {
        self.session.as_ref()?.pos_of_span_hyp(hyp, (l, r))
    }

    pub fn pos_of_span_goal(&self, l: usize, r: usize) -> Option<usize> {
        self.session.as_ref()?.pos_of_span_goal((l, r))
    }

    pub fn check(&self, query: &str) -> String {
        let eng = if let Some(s) = &self.session {
            s.initial_engine()
        } else {
            return "No session".to_string();
        };
        match eng.check(query) {
            Ok(r) => r,
            Err(e) => format!("{:?}", e),
        }
    }

    pub fn notation_list(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&notation_list()).unwrap()
    }

    pub fn action_of_tactic(&self, tactic: &str) -> JsValue {
        let s = if let Some(s) = &self.session {
            s
        } else {
            return JsValue::from_str("No session");
        };
        let r = s.action_of_tactic(tactic);
        serde_wasm_bindgen::to_value(&r).unwrap()
    }

    pub fn search(&self, query: &str) -> JsValue {
        let eng = if let Some(s) = &self.session {
            s.initial_engine()
        } else {
            return JsValue::from_str("No session");
        };
        match eng.search(query) {
            Ok(r) => {
                let x = r
                    .into_iter()
                    .map(|x| {
                        let ty = eng.calc_type_and_infer(&x).unwrap();
                        (x, eng.pretty_print(&ty))
                    })
                    .collect::<Vec<_>>();
                serde_wasm_bindgen::to_value(&x).unwrap()
            }
            Err(e) => JsValue::from_str(&format!("{:?}", e)),
        }
    }
}
