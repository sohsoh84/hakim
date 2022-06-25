Import /Arith.

Axiom divide_unfold: ∀ a b: ℤ, a | b -> ∃ c: ℤ, a * c = b.
Axiom divide_fold: ∀ a b: ℤ, (∃ c: ℤ, a * c = b) -> a | b.
Suggest hyp default apply divide_unfold in $n; a | b => ∃ c, a * c = b.
Suggest goal default apply divide_fold; a | b => ∃ c, a * c = b.

Theorem divide_le: ∀ a: ℤ, 1 ≤ a -> ∀ b: ℤ, b | a -> b ≤ a.
Proof.
    intros.
    add_hyp (1 ≤ b ∨ b < 1).
    lia.
    destruct H1 with (or_ind ? ?).
    lia.
    apply divide_unfold in H0.
    destruct H0 with (ex_ind ? ?) to (c c_property).
    apply eq_sym in c_property.
    rewrite c_property.
    add_hyp (1 ≤ c).
    apply NNPP.
    intros.
    apply not_le in H0.
    add_hyp (c = 0 ∨ c < 0).
    lia.
    destruct H2 with (or_ind ? ?).
    remove_hyp H0.
    add_hyp (0 < -a).
    rewrite c_property.
    replace #1 (- (b * c)) with ( (b *- c)).
    lia.
    apply zero_lt_mult_pos.
    lia.
    lia.
    lia.
    replace #1 (c) with (0) in c_property.
    assumption.
    lia.
    revert H1.
    remove_hyp c_property.
    revert b.
    apply z_induction_simple.
    intros.
    lia.
    lia.
Qed.

Theorem divide_refl: ∀ a: ℤ, a | a.
Proof.
    intros.
    apply divide_fold.
    apply (ex_intro ? ? (1)).
    lia.
Qed.
Todo divide_trans: ∀ a b c: ℤ, a | b -> b | c -> a | c.
Theorem divide_0: ∀ a: ℤ, a | 0.
Proof.
    intros.
    apply divide_fold.
    apply (ex_intro ? ? (0)).
    lia.
Qed.
Theorem divide_1_positive: ∀ a: ℤ, 0 < a -> a | 1 -> a = 1.
Proof.
    intros.
    apply divide_unfold in H0.
    destruct H0 with (ex_ind ? ?) to (c c_property).
    add_hyp (1 < a -> False).
    Switch 1.
    lia.
    intros.
    add_hyp (0 < c).
    apply NNPP.
    intros.
    add_hyp (0 = c ∨ c < 0).
    lia.
    destruct H2 with (or_ind ? ?).
    add_hyp (a * c < 0).
    replace #1 (a * c) with (c * a).
    lia.
    replace #1 (0) with (c * 0).
    lia.
    apply lt_multiply_negative.
    assumption.
    assumption.
    lia.
    replace #1 (c) with (0) in c_property.
    auto_set.
    lia.
    add_hyp (c * 1 < c * a).
    apply lt_multiply_positive.
    assumption.
    assumption.
    replace #1 (c * a) with (1) in H2.
    lia.
    lia.
Qed.
Todo divide_factor: ∀ a b c: ℤ, a | b -> a | b * c.
Todo divide_plus: ∀ a b c: ℤ, a | b -> a | c -> a | b + c.
Todo divide_minus: ∀ a b c: ℤ, a | b -> a | b + c -> a | c.
Theorem divide_linear_combination: ∀ a b c: ℤ, a | b -> a | c -> (∀ k l: ℤ, a | k * b + l * c).
Proof.
    intros.
    apply divide_plus.
    replace #1 (l * c) with (c * l).
    lia.
    apply divide_factor.
    assumption.
    replace #1 (k * b) with (b * k).
    lia.
    apply divide_factor.
    assumption.
Qed.

Axiom prime: ℤ -> U.
Axiom prime_unfold: ∀ x: ℤ, prime x -> 1 < x ∧ (∀ y: ℤ, 0 < y -> y | x -> y = 1 ∨ y = x).
Axiom prime_fold: ∀ x: ℤ, 1 < x ∧ (∀ y: ℤ, 0 < y -> y | x -> y = 1 ∨ y = x) -> prime x.
Todo contpos_prime_fold: ∀ x: ℤ, ~ prime x -> 1 < x -> ∃ y: ℤ, 1 < y ∧ y < x ∧ y | x. 
Todo prime_ge_2: ∀ x: ℤ, prime x -> 2 ≤ x.
Todo prime_is_positive: ∀ x: ℤ, prime x -> 0 < x.
Theorem prime_divisor_for_positive: ∀ x: ℤ, 1 < x -> ∃ p: ℤ, prime p ∧ p | x.
Proof.
    intros.
    add_hyp (2 ≤ x).
    lia.
    remove_hyp H.
    revert H0.
    revert x.
    apply z_induction_strong.
    intros.
    add_hyp (prime n ∨ ~ prime n).
    assumption.
    revert H1.
    intros.
    destruct H1 with (or_ind ? ?).
    apply contpos_prime_fold in H1.
    Seq (add_hyp (⁨1 < n⁩)) (remove_hyp H1) (Switch 1) (add_hyp H1_o := (H1 H2)) (remove_hyp H2) (remove_hyp H1) .
    destruct H1_o with (ex_ind ? ?) to (y y_property).
    add_hyp H0_ex := (H0 (y)).
    Seq (add_hyp (⁨2 ≤ y⁩)) (remove_hyp H0_ex) (Switch 1) (add_hyp H0_ex_o := (H0_ex H1)) (remove_hyp H1) (remove_hyp H0_ex) .
    Seq (add_hyp (⁨y < n⁩)) (remove_hyp H0_ex_o) (Switch 1) (add_hyp H0_ex_o_o := (H0_ex_o H1)) (remove_hyp H1) (remove_hyp H0_ex_o) .
    destruct H0_ex_o_o with (ex_ind ? ?) to (p p_property).
    apply (ex_intro ? ? (p)).
    apply and_intro.
    apply (⁨divide_trans ?0 y ?4 ?6 ?8⁩).
    assumption.
    assumption.
    assumption.
    assumption.
    lia.
    lia.
    apply (ex_intro ? ? (n)).
    apply and_intro.
    apply divide_refl.
    assumption.
Qed.

Import /ProductOperator.
Axiom divide_multi:   ∀ A: set ℤ, ∀ a : ℤ, a ∈ A -> a | multi A.
