(require [darcy.vec :as v])
(require [darcy.math :as math])

;; Exponential Moving Average
;; prices: vector of prices, sorted Newest (index 0) to Oldest.
;; period: number of periods for the EMA.
(defn ema [prices:vec<f64> period:usize]
  (let [len (v/len prices)]
    (if (math/lt len period)
      0.0 ; Not enough data
      (let [k (/ 2.0 (+ (cast period f64) 1.0))
            last-idx (- period 1)
            ;; Seed with the oldest price in the window
            start-val (v/get prices last-idx)]
        
        (let [acc start-val]
          (do
            (for i (range 1 period)
               (let [idx (- last-idx i)
                     price (v/get prices idx)]
                 (let! acc (+ (* price k) (* acc (- 1.0 k))))))
            acc))))))
