(defstruct Order
  (id Uuid)
  (qty u32)
  (price f64))

(defn total [o:Order]
  (* o.qty o.price))
