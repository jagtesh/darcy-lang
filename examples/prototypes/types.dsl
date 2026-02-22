(defenum signal
  (buy)
  (sell)
  (hold))

(defrecord bar
  (open f64)
  (high f64)
  (low f64)
  (close f64)
  (volume i64))
