use crate::{
    app_ref,
    brain::{Term, TermRef},
    interactive::Frame,
    library::prelude::set,
    term_ref, Abstraction,
};

use super::{get_one_arg, Error::*, Result};

#[derive(Debug, Clone)]
enum EnsembleTree {
    Set(u16),
    Union(Box<EnsembleTree>, Box<EnsembleTree>),
    Intersection(Box<EnsembleTree>, Box<EnsembleTree>),
    Setminus(Box<EnsembleTree>, Box<EnsembleTree>),
    Eq(Box<EnsembleTree>, Box<EnsembleTree>),
    Included(Box<EnsembleTree>, Box<EnsembleTree>),
    Inset(u16, Box<EnsembleTree>),
    Outset(u16, Box<EnsembleTree>),
}
use EnsembleTree::*;

use std::collections::HashMap;
use std::collections::VecDeque;
struct Identifier {
    map: HashMap<TermRef, u16>,
    idCounter: u16,
}
impl Identifier {
    pub fn new() -> Identifier {
        Identifier {
            map: HashMap::new(),
            idCounter: 1,
        }
    }
    pub fn get(&mut self, v: &TermRef) -> u16 {
        match self.map.get(v) {
            Some(&x) => x,
            None => {
                self.map.insert(v.clone(), self.idCounter);
                self.idCounter += 1;
                return self.idCounter - 1;
            }
        }
    }
}
fn add_hyp(
    h: &EnsembleTree,
    undo: bool,
    simple_hyps: &mut HashMap<(u16, u16), i32>,
    ahyps: &mut VecDeque<EnsembleTree>,
    bhyps: &mut VecDeque<EnsembleTree>,
) -> i32 {
    let add = |hyps: &mut VecDeque<EnsembleTree>| -> i32 {
        if undo {
            hyps.pop_back();
        } else {
            hyps.push_back(h.clone());
        }
        0
    };
    match h.clone() {
        Inset(x, X) => {
            match *X {
                Intersection(..) | Setminus(..) => add(ahyps),
                Union(..) => add(bhyps),
                Set(i) => {
                    let counter = simple_hyps.entry((x, i)).or_insert(0);
                    if undo {
                        (*counter) -= 1;
                    } else {
                        if *counter < 0 {
                            return 1; //found Contradiction
                        }
                        (*counter) += 1;
                    }
                    0
                }
                _ => -1, //false input
            }
        }
        Outset(x, X) => {
            match *X {
                Union(..) => add(ahyps),
                Intersection(..) | Setminus(..) => add(bhyps),
                Set(i) => {
                    let counter = simple_hyps.entry((x, i)).or_insert(0);
                    if undo {
                        (*counter) += 1;
                    } else {
                        if *counter > 0 {
                            return 1; //found Contradiction
                        }
                        (*counter) -= 1;
                    }
                    0
                }
                _ => -1, //false input
            }
        }
        Eq(..) => add(ahyps),
        Included(..) => add(bhyps),
        _ => -1,
    }
}
fn from_set_type(t: &TermRef, sets_id: &mut Identifier) -> EnsembleTree {
    if let Term::App { func, op: op2 } = t.as_ref() {
        if let Term::App { func, op: op1 } = func.as_ref() {
            if let Term::App { func, op: _ } = func.as_ref() {
                if let Term::Axiom { unique_name, .. } = func.as_ref() {
                    if unique_name == "union" {
                        return Union(
                            Box::new(from_set_type(op1, sets_id)),
                            Box::new(from_set_type(op2, sets_id)),
                        );
                    }
                    if unique_name == "intersection" {
                        return Intersection(
                            Box::new(from_set_type(op1, sets_id)),
                            Box::new(from_set_type(op2, sets_id)),
                        );
                    }
                    if unique_name == "setminus" {
                        return Setminus(
                            Box::new(from_set_type(op1, sets_id)),
                            Box::new(from_set_type(op2, sets_id)),
                        );
                    }
                }
            }
        }
    }
    Set(sets_id.get(t))
}
fn from_prop_type(
    t: &TermRef,
    elements_id: &mut Identifier,
    sets_id: &mut Identifier,
) -> Option<(EnsembleTree, TermRef)> {
    if let Term::Forall(a) = t.as_ref() {
        if let Term::Axiom { unique_name, .. } = a.body.as_ref() {
            if unique_name == "False" {
                if let Some((Inset(x, A), ty)) = from_prop_type(&a.var_ty, elements_id, sets_id) {
                    return Some((Outset(x, A), ty));
                }
            }
            //the Included -> false or eq -> false type prop is one work
        }
    }

    if let Term::App { func, op: op2 } = t.as_ref() {
        if let Term::App { func, op: op1 } = func.as_ref() {
            if let Term::App { func, op: ty } = func.as_ref() {
                if let Term::Axiom { unique_name, .. } = func.as_ref() {
                    if unique_name == "inset" {
                        let tree =
                            Inset(elements_id.get(op1), Box::new(from_set_type(op2, sets_id)));
                        return Some((tree, term_ref!(app_ref!(set(), ty))));
                    }
                    if unique_name == "included" {
                        let tree = Included(
                            Box::new(from_set_type(op1, sets_id)),
                            Box::new(from_set_type(op2, sets_id)),
                        );
                        return Some((tree, term_ref!(app_ref!(set(), ty))));
                    }
                    if unique_name == "eq" {
                        let tree = Eq(
                            Box::new(from_set_type(op1, sets_id)),
                            Box::new(from_set_type(op2, sets_id)),
                        );
                        return Some((tree, ty.clone()));
                    }
                }
            }
        }
    }
    None
}
fn dfs(
    goal: &EnsembleTree,
    element_in_goal: u16,
    simple_hyps: &mut HashMap<(u16, u16), i32>,
    ahyps: &mut VecDeque<EnsembleTree>,
    bhyps: &mut VecDeque<EnsembleTree>,
) -> i32 {
    println!("{} {:?}", element_in_goal, goal);
    // pass to dfs main x; Inset() ? Outset()
    let mut ans = 0;
    let mut step1 = |h, g, x| {
        let c = add_hyp(h, false, simple_hyps, ahyps, bhyps);
        if c == 0 {
            ans = dfs(g, x, simple_hyps, ahyps, bhyps);
            add_hyp(h, true, simple_hyps, ahyps, bhyps);
        } else {
            ans = c;
        }
    };
    match goal.clone() {
        Inset(x, X) => {
            if let Union(A, B) = *X {
                step1(&Outset(x, B), &Inset(x, A), x);
            }
        }
        Outset(x, A) => {
            step1(&Inset(x, A), &Set(0), x);
        }
        Included(A, B) => {
            //bigest u16 value for new id
            step1(&Inset(65535, A), &Inset(65535, B), 65535);
        }
        _ => (),
    }
    if ans != 0 {
        return ans;
    }
    if let Some(h) = ahyps.pop_front() {
        let mut step2 = |h1, h2| -> () {
            let c1 = add_hyp(h1, false, simple_hyps, ahyps, bhyps);
            if c1 == 0 {
                let c2 = add_hyp(h2, false, simple_hyps, ahyps, bhyps);
                if c2 == 0 {
                    ans = dfs(goal, element_in_goal, simple_hyps, ahyps, bhyps);
                    add_hyp(h2, true, simple_hyps, ahyps, bhyps);
                } else {
                    ans = c2;
                }
                add_hyp(h1, true, simple_hyps, ahyps, bhyps);
            } else {
                ans = c1;
            }
        };
        if let Inset(x, X) = h {
            if let Intersection(A, B) = *X {
                step2(&Inset(x, A.clone()), &Inset(x, B.clone()));
            } else if let Setminus(A, B) = *X {
                step2(&Inset(x, A.clone()), &Outset(x, B));
            }
        } else if let Outset(x, X) = h {
            if let Union(A, B) = *X {
                step2(&Outset(x, A), &Outset(x, B));
            }
        } else if let Eq(A, B) = h {
            step2(&Included(A.clone(), B.clone()), &Included(B, A));
        }
    }
    if ans != 0 {
        return ans;
    }
    let mut step3 = |g1, g2, x| {
        let c = dfs(g1, x, simple_hyps, ahyps, bhyps);
        if c == 1 {
            ans = dfs(g2, x, simple_hyps, ahyps, bhyps);
        } else {
            ans = c;
        }
    };
    match goal.clone() {
        Inset(x, X) => match *X {
            Intersection(A, B) => {
                step3(&Inset(x, A), &Inset(x, B), x);
            }
            Setminus(A, B) => {
                step3(&Inset(x, A), &Outset(x, B), x);
            }
            Set(i) => {
                if let Some(counter) = simple_hyps.get(&(x, i)) {
                    if *counter > 0 {
                        return 1;
                    }
                }
            }
            _ => (),
        },
        Eq(A, B) => {
            step3(
                &Included(A.clone(), B.clone()),
                &Included(B, A),
                element_in_goal,
            );
        }
        _ => (),
    }
    if ans != 0 {
        return ans;
    }
    if let Some(h) = bhyps.pop_front() {
        let mut step4 = |h1, h2| {
            let c = add_hyp(h1, false, simple_hyps, ahyps, bhyps);
            if c == 0 {
                let c = dfs(goal, element_in_goal, simple_hyps, ahyps, bhyps);
                add_hyp(h1, true, simple_hyps, ahyps, bhyps);

                if c == 1 {
                    let c = add_hyp(h2, false, simple_hyps, ahyps, bhyps);
                    if c == 0 {
                        ans = dfs(goal, element_in_goal, simple_hyps, ahyps, bhyps);
                        add_hyp(h2, true, simple_hyps, ahyps, bhyps);
                    } else {
                        ans = c;
                    }
                } else {
                    ans = c;
                }
            } else {
                ans = c;
            }
        };
        if let Inset(x, X) = h {
            if let Union(A, B) = *X.clone() {
                step4(&Inset(x, A), &Inset(x, B));
            }
        } else if let Outset(x, X) = h {
            if let Intersection(A, B) = *X.clone() {
                step4(&Outset(x, A), &Outset(x, B));
            } else if let Setminus(A, B) = *X.clone() {
                step4(&Outset(x, A), &Inset(x, B));
            }
        } else if let Included(A, B) = h {
            //can we add element_in_goal ∈ A too but no need
            step4(&Inset(element_in_goal, B), &Outset(element_in_goal, A));
        }
    }
    return ans;
}
pub fn auto_set(mut frame: Frame) -> Result<Vec<Frame>> {
    let mut elements_id: Identifier = Identifier::new();
    let mut sets_id: Identifier = Identifier::new();

    if let Some((goal, goal_ty)) = from_prop_type(&frame.goal, &mut elements_id, &mut sets_id) {
        //map simple form of x ∈ A to (x, A) -> {number of existen prop of this type}
        //minus number mean we have prop of x ∉ A
        let mut simple_hyps: HashMap<(u16, u16), i32> = HashMap::new();
        //hypothiss that generate two goal
        let mut bhyps: VecDeque<EnsembleTree> = VecDeque::new();
        //hyps that not
        let mut ahyps: VecDeque<EnsembleTree> = VecDeque::new();

        for val in frame.hyps.values() {
            if let Some((x, ty)) = from_prop_type(val, &mut elements_id, &mut sets_id) {
                if goal_ty == ty
                    && add_hyp(&x, false, &mut simple_hyps, &mut ahyps, &mut bhyps) != 0
                {
                    return Err(BadHyp("can,t match", val.clone()));
                }
            }
        }
        let ans = dfs(&goal, 65535, &mut simple_hyps, &mut ahyps, &mut bhyps);
        if ans == 1 {
            return Ok(vec![]);
        } else if ans == 0 {
            return Err(CanNotSolve("auto_set"));
        } else if ans == -1 {
            return Err(BadGoal("can,t match hyp"));
        }
    }
    Err(BadGoal("can,t solve this type"))
}
