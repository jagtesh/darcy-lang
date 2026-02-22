(require [darcy.vec :as v])
(require [darcy.io :as io])
(require [darcy.rand :as rand])
(require [darcy.math :as math])

(defenum signal
  (buy)
  (sell)
  (hold))

(defn ema [prices:vec<f64> period:usize]
  (let [len (v/len prices)]
    (if (math/lt len period)
      0.0
      (let [k (/ 2.0 (+ (cast period f64) 1.0))
            last-idx (- period 1)
            start-val (v/get prices last-idx)]
        (let [acc start-val]
          (do
            (for i (range 1 period)
               (let [idx (- last-idx i)
                     price (v/get prices idx)]
                 (let! acc (+ (* price k) (* acc (- 1.0 k))))))
            acc))))))

(defn run-strategy [prices:vec<f64>]
  (let [fast-period 10
        slow-period 21
        fast (ema prices fast-period)
        slow (ema prices slow-period)]
    (cond
      ((math/gt fast slow) (buy))
      ((math/lt fast slow) (sell))
      (else (hold)))))

(defn reverse-vec [input:vec<f64>]
  (let [output (vec<f64>)
        len (v/len input)]
    (do
      (for i (range 0 len)
        (let [idx (- (- len 1) i)]
          (v/push output (v/get input idx))))
      output)))

(defn generate-prices [n:i64]
  (do
    (rand/seed 999)
    (let [prices (vec<f64>)
          i 0
          price 100.0]
      (do
        (loop
           (while (math/lt i n)
             (do
                (v/push prices price)
                (let [change (* (- (rand/rand-f64) 0.5) 5.0)]
                   (let! price (+ price change)))
                (let! i (+ i 1)))))
        prices))))

(defn main []
  (do
    (io/dbg "Generating 100 bars of price data...")
    (let [raw-prices (generate-prices 100)
          market-data (reverse-vec raw-prices)]
      (do
        (io/dbg "Market Data (latest 5):")
        (let [preview (v/take market-data 5)]
           (io/dbg preview))
    
        (io/dbg "Running Strategy...")
        (let [sig (run-strategy market-data)]
           (case sig
             (buy (io/dbg "Result: BUY"))
             (sell (io/dbg "Result: SELL"))
             (hold (io/dbg "Result: HOLD"))
             (_ (io/dbg "Result: ???"))))))))
