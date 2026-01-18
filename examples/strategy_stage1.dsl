; Stage 1: core DSL features (structs, unions, match, vectors, dbg)

(defstruct candle
  (close f64)
  (volume f64))

(defunion signal
  (buy (strength f64))
  (sell (strength f64))
  (hold))

(defn decision [s:signal]
  (match s
    (buy (strength k) k)
    (sell (strength k) (* k -1.0))
    (hold 0.0)))

(defn closes [cs:vec<candle>]
  cs.close)

(defn main []
  (dbg (decision (buy 0.7))))

(defn demo-closes []
  (dbg (closes [(candle 101.25 1200.0) (candle 99.75 980.0)])))
