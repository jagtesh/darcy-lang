(defstruct order
  (qty u32)
  (price f64))

(defstruct invoice
  (qty u32)
  (price f64))

(defn total [o]
  (* o.qty o.price))
