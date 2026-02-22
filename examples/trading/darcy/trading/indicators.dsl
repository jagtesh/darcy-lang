(defn sma [values:vec<f64> period:i32]
  (let [window (darcy.vec/take values period)
        total (darcy.vec/fold (fn [acc x] (+ acc x)) 0.0 window)]
    (/ total period)))
