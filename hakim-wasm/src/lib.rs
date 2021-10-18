mod utils;

use hakim_engine::engine::{Engine, interactive::InteractiveSession};
use utils::set_panic_hook;
use wasm_bindgen::prelude::*;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
extern {
    fn alert(s: &str);
}

#[wasm_bindgen]
pub struct Instance {
    session: InteractiveSession<'static>,
}

#[wasm_bindgen(start)]
pub fn start() {
    set_panic_hook();
}

#[wasm_bindgen]
impl Instance {
    #[wasm_bindgen(constructor)]
    pub fn new(goal: &str) -> Self {
        let engine = Box::leak(Box::new(Engine::default()));
        let session = engine.interactive_session(goal);
        Instance { session }
    }

    #[wasm_bindgen]
    pub fn monitor(&self) -> String {
        self.session.monitor_string()
    }

    #[wasm_bindgen]
    pub fn run_tactic(&mut self, tactic: &str) -> Option<String> {
        match self.session.run_tactic(tactic) {
            Ok(_) => None,
            Err(e) => Some(format!("{:?}", e)),
        }
    }
}
