(defrecord order
  [qty:u32]
  [price:f64])
 
(defrecord invoice
  [qty:u32]
  [price:f64])
 
(defn total [o]
  (* o.qty o.price))
