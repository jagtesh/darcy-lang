(require [darcy.math :as math])
(require [types :as types])
(require [ema])

;; Strategy: Dual EMA Crossover
;; Returns a Signal (Buy, Sell, or Hold) based on the latest state.
(defn run-strategy [prices:vec<f64>]
  (let [fast-period 10
        slow-period 21
        fast (ema/ema prices fast-period)
        slow (ema/ema prices slow-period)]
    
    (cond
      ((math/gt fast slow) (types/buy))
      ((math/lt fast slow) (types/sell))
      (else (types/hold)))))
