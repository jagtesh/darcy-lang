(defstruct Order
  (qty u32)
  (price f64))

(defstruct Invoice
  (qty u32)
  (price f64))

(defn total [o]
  (* o.qty o.price))
