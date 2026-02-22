(require [trading.types :refer [signal]])
(require [trading.signals :as sig])
(require [darcy.astra.sim :as sim])

(extern "crate::round_tick" (defn round-tick [x:f64] f64))

(defn generate-signal [bars:vec<darcy.astra.sim/bar> fast:i32 slow:i32]
  (let [closes bars.close
        rounded (darcy.vec/map (fn [x] (round-tick x)) closes)]
    (sig/signal-from-closes rounded fast slow)))

(defn gen-bars [n:i64 seed:i64 start:f64]
  (sim/gen-bars n seed start))
