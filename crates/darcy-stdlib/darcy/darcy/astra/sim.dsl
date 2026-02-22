(require [darcy.core])
(require [darcy.math])
(require [darcy.vec])
(require [darcy.rand :as rand])

(defrecord bar
  (ts i64)
  (open f64)
  (high f64)
  (low f64)
  (close f64)
  (volume f64))

(defrecord sim-state
  (rng rand/rand-state)
  (bars vec<bar>)
  (price f64))

(defn step [state:sim-state idx:i64]
  (let [pair (rand/rand-normal state.rng)
        rng1 pair.state
        z pair.value
        drift 0.00005
        vol 0.002
        ret (+ drift (* z vol))
        open state.price
        close (* open (+ 1.0 ret))
        wick-pair (rand/rand-f64 rng1)
        rng2 wick-pair.state
        u wick-pair.value
        wick (* u (* open 0.001))
        high (darcy.math/max open (+ close wick))
        low (darcy.math/min open (- close wick))
        vol-pair (rand/rand-f64 rng2)
        rng3 vol-pair.state
        v vol-pair.value
        volume (+ 800.0 (* 400.0 v))
        bar (bar idx open high low close volume)
        bars2 (darcy.vec/push (darcy.core/clone state.bars) bar)]
    (sim-state rng3 bars2 close)))

(defn gen-bars [n:usize seed-val:i64 start:f64]
  (let [init (sim-state (rand/seed seed-val) (darcy.vec/new) start)
        out (darcy.vec/fold (fn [st i] (step st (cast i i64))) init (darcy.vec/range n))]
    out.bars))
