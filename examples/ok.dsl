(defrecord order
  [id:string]
  [qty:u32]
  [price:f64])
 
(defn total [o:order]
  (* o.qty o.price))
