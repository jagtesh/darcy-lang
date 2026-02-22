(require [darcy.vec :as v])
(require [darcy.io :as io])
(require [darcy.rand :as rand])
(require [darcy.math :as math])
(require [strategy])
(require [types :as types])

(defn generate-prices [n:i64]
  (do
    (rand/seed 999)
    (let [prices (vec<f64>)]
      (let [i 0]
        (let [price 100.0]
          (do
            (loop
               (while (math/lt i n)
                 (do
                    (v/push prices price)
                    (let [change (* (- (rand/rand-f64) 0.5) 5.0)]
                       (let! price (+ price change)))
                    (let! i (+ i 1)))))
            prices))))))

(defn main []
  (do
    (io/dbg "Generating 100 bars of price data...")
    (let [raw-prices (generate-prices 100)]
      (do
        (io/dbg "Market Data (latest 5):")
        (let [preview (v/take raw-prices 5)]
           (io/dbg preview))
    
        (io/dbg "Running Strategy...")
        (let [sig (strategy/run-strategy raw-prices)]
           (case sig
             (types/buy (io/dbg "Result: BUY"))
             (types/sell (io/dbg "Result: SELL"))
             (types/hold (io/dbg "Result: HOLD"))
             (_ (io/dbg "Result: ???"))))))))
