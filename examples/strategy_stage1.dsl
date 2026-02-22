; Stage 1: core DSL features (structs, unions, case, vectors, dbg)

(defrecord candle
  (close f64)
  (volume f64))
 
(defenum signal
  (buy (strength f64))
  (sell (strength f64))
  (hold))
 
(defn decision [s:signal]
  (case s
    (buy (strength k) k)
    (sell (strength k) (* k -1.0))
    (hold 0.0)))
 
(defn closes [cs:vec<candle>]
  cs.close)
 
(defn main []
  (darcy.io/dbg (decision (buy 0.7))))

(defn demo-closes []
  (darcy.io/dbg (closes [(candle 101.25 1200.0) (candle 99.75 980.0)])))
