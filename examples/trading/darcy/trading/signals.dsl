(require [trading.types :refer [signal]])
(require [trading.indicators :as ind])

(defn crossover [fast:f64 slow:f64]
  (cond
    ((darcy.math/gt fast slow) (signal/buy))
    ((darcy.math/lt fast slow) (signal/sell))
    (else (signal/hold))))

(defn signal-from-closes [closes:vec<f64> fast:i32 slow:i32]
  (let [fast-sma (ind/sma closes fast)
        slow-sma (ind/sma closes slow)]
    (crossover fast-sma slow-sma)))
