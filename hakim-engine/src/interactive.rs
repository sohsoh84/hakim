use im::vector;
use serde::{Deserialize, Serialize};

use crate::brain::{contains_wild, predict_axiom, TermRef};

use crate::engine::{Engine, Error};

#[cfg(test)]
mod tests;

mod suggest;
pub mod tactic;

use tactic::{add_hyp, apply, intros, lia, replace, rewrite, ring};

use self::suggest::{
    suggest_on_goal, suggest_on_goal_dblclk, suggest_on_hyp, suggest_on_hyp_dblclk,
};

pub use self::suggest::Suggestion;
use self::tactic::{auto_set, remove_hyp};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frame {
    pub goal: TermRef,
    pub hyps: im::HashMap<String, TermRef>,
    pub engine: Engine,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub frames: im::Vector<Frame>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct HistoryRecord {
    tactic: String,
    snapshot: Snapshot,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Session {
    history: im::Vector<HistoryRecord>,
}

fn smart_split(text: &str) -> Vec<String> {
    let mut r = vec![];
    let mut s = "".to_string();
    let mut d = 0;
    for c in text.chars() {
        if c == '(' {
            d += 1;
        }
        if c == ')' {
            d -= 1;
        }
        if d != 0 {
            s.push(c);
            continue;
        }
        if c.is_whitespace() {
            if !s.is_empty() {
                r.push(s);
                s = "".to_string();
            }
            continue;
        }
        s.push(c);
    }
    if !s.is_empty() {
        r.push(s);
    }
    r
}

impl Session {
    pub fn new(engine: Engine, goal: &str) -> Result<Self, Error> {
        let snapshot = Snapshot::new(engine, goal)?;
        let hr = HistoryRecord {
            snapshot,
            tactic: "Goal".to_string(),
        };
        Ok(Session {
            history: vector![hr],
        })
    }

    pub fn initial_engine(&self) -> Engine {
        self.history[0].snapshot.frames[0].engine.clone()
    }

    pub fn last_snapshot(&self) -> &Snapshot {
        &self.history.last().unwrap().snapshot
    }

    pub fn run_tactic(&mut self, line: &str) -> Result<(), tactic::Error> {
        if line.trim() == "Undo" {
            return self.undo();
        }
        if let Some(x) = line.strip_prefix("Switch ") {
            let t: usize = x.parse().map_err(|_| tactic::Error::BadArg {
                arg: x.to_string(),
                tactic_name: "Switch".to_string(),
            })?;
            let snapshot = self.last_snapshot().switch_frame(t);
            self.history.push_back(HistoryRecord {
                tactic: line.to_string(),
                snapshot,
            });
            return Ok(());
        }
        let snapshot = self.last_snapshot().run_tactic(line)?;
        self.history.push_back(HistoryRecord {
            tactic: line.to_string(),
            snapshot,
        });
        Ok(())
    }

    pub fn run_suggestion(
        &mut self,
        sugg: Suggestion,
        ans: Vec<String>,
    ) -> Result<(), tactic::Error> {
        assert_eq!(sugg.questions.len(), ans.len());
        let tactics = sugg.tactic.into_iter().map(|mut x| {
            for (i, a) in ans.iter().enumerate() {
                x = x.replace(&format!("${}", i), a);
            }
            x
        });
        for t in tactics {
            self.run_tactic(&t)?;
        }
        Ok(())
    }

    pub fn monitor_string(&self) -> String {
        self.last_snapshot().monitor_string()
    }

    pub fn print(&self) {
        println!("{}", self.monitor_string());
    }

    pub fn is_finished(&self) -> bool {
        self.last_snapshot().is_finished()
    }

    pub fn undo(&mut self) -> Result<(), tactic::Error> {
        if self.history.len() <= 1 {
            return Err(tactic::Error::CanNotUndo);
        }
        self.history.pop_back();
        Ok(())
    }

    pub fn get_history(&self) -> Vec<String> {
        self.history.iter().map(|x| x.tactic.clone()).collect()
    }

    pub fn suggest_on_goal_dblclk(&self) -> Option<Suggestion> {
        let frame = self.last_snapshot().clone().pop_frame();
        frame.suggest_on_goal_dblclk()
    }

    pub fn suggest_on_goal_menu(&self) -> Vec<Suggestion> {
        let frame = self.last_snapshot().clone().pop_frame();
        frame.suggest_on_goal_menu()
    }

    pub fn suggest_on_hyp_dblclk(&self, hyp_name: &str) -> Option<Suggestion> {
        let frame = self.last_snapshot().clone().pop_frame();
        frame.suggest_on_hyp_dblclk(hyp_name)
    }

    pub fn suggest_on_hyp_menu(&self, hyp_name: &str) -> Vec<Suggestion> {
        let frame = self.last_snapshot().clone().pop_frame();
        frame.suggest_on_hyp_menu(hyp_name)
    }
}

impl Snapshot {
    pub fn new(engine: Engine, goal: &str) -> Result<Snapshot, Error> {
        let goal_term = engine.parse_text(goal)?;
        if contains_wild(&goal_term) {
            return Err(Error::GoalWithWildCard(goal_term));
        }
        let frame = Frame {
            hyps: Default::default(),
            goal: goal_term,
            engine,
        };
        Ok(Snapshot {
            frames: vector![frame],
        })
    }

    pub fn monitor_string(&self) -> String {
        if self.is_finished() {
            return "No more subgoals.".to_string();
        }
        let goal_count = self.frames.len();
        let mut r = "".to_string();
        for (name, ty) in &self.frames.last().unwrap().hyps {
            r += &format!(" {}: {:#?}\n", name, ty);
        }
        for (i, frame) in self.frames.iter().rev().enumerate() {
            r += &format!(
                "--------------------------------------------({}/{})\n",
                i + 1,
                goal_count
            );
            r += &format!("    {:#?}\n", frame.goal);
        }
        r
    }

    pub fn run_tactic(&self, line: &str) -> Result<Self, tactic::Error> {
        let mut snapshot = self.clone();
        let frame = snapshot.pop_frame();
        let new_frames = frame.run_tactic(line)?;
        snapshot.frames.extend(new_frames);
        Ok(snapshot)
    }

    pub fn pop_frame(&mut self) -> Frame {
        self.frames.pop_back().unwrap()
    }

    fn switch_frame(&self, i: usize) -> Self {
        let mut result = self.clone();
        result
            .frames
            .swap(result.frames.len() - 1, result.frames.len() - 1 - i);
        result
    }

    pub fn is_finished(&self) -> bool {
        self.frames.is_empty()
    }
}

impl Frame {
    pub fn add_hyp_with_name(&mut self, name: &str, ty: TermRef) -> tactic::Result<()> {
        self.engine.add_axiom_with_term(name, ty.clone())?;
        self.hyps.insert(name.to_string(), ty);
        Ok(())
    }

    pub fn remove_hyp_with_name(&mut self, name: String) -> tactic::Result<TermRef> {
        for (_, hyp) in &self.hyps {
            if predict_axiom(hyp, &|x| x == name) {
                return Err(tactic::Error::ContextDependOnHyp(name, hyp.clone()));
            }
        }
        if predict_axiom(&self.goal, &|x| x == name) {
            return Err(tactic::Error::ContextDependOnHyp(name, self.goal.clone()));
        }
        if let Some(hyp) = self.hyps.remove(&name) {
            self.engine.remove_name_unchecked(&name);
            return Ok(hyp);
        }
        Err(tactic::Error::UnknownHyp(name))
    }

    pub fn suggest_on_goal_dblclk(&self) -> Option<Suggestion> {
        suggest_on_goal_dblclk(&self.goal, &self.engine)
    }

    pub fn suggest_on_goal_menu(&self) -> Vec<Suggestion> {
        suggest_on_goal(&self.goal, &self.engine)
    }

    pub fn suggest_on_hyp_dblclk(&self, hyp_name: &str) -> Option<Suggestion> {
        let h = self.hyps.get(hyp_name)?;
        suggest_on_hyp_dblclk(&self.engine, hyp_name, h)
    }

    pub fn suggest_on_hyp_menu(&self, hyp_name: &str) -> Vec<Suggestion> {
        let h = if let Some(x) = self.hyps.get(hyp_name) {
            x
        } else {
            return vec![];
        };
        suggest_on_hyp(&self.engine, hyp_name, h)
    }

    pub fn run_tactic(&self, line: &str) -> Result<Vec<Self>, tactic::Error> {
        let parts = smart_split(line);
        let mut parts = parts.into_iter();
        let name = parts.next().ok_or(tactic::Error::EmptyTactic)?;
        let frame = self.clone();
        match name.as_str() {
            "intros" => intros(frame, parts),
            "rewrite" => rewrite(frame, parts),
            "replace" => replace(frame, parts),
            "apply" => apply(frame, parts),
            "add_hyp" => add_hyp(frame, parts),
            "remove_hyp" => remove_hyp(frame, parts),
            "ring" => ring(frame),
            "lia" => lia(frame),
            "auto_set" => auto_set(frame),
            _ => Err(tactic::Error::UnknownTactic(name.to_string())),
        }
    }
}
