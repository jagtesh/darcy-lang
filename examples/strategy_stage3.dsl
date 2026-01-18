; Stage 3: multi-asset portfolio sketch with vector broadcasting
 
(defrecord position
  (asset i32)
  (weight f64))
 
(defenum bias
  (risk-on)
  (risk-off))
 
(defn apply-bias [b:bias weights:vec<f64>]
  (case b
    (risk-on (* weights 1.1))
    (risk-off (* weights 0.6))))
 
(defn weights [ps:vec<position>]
  ps.weight)
 
(defn main []
  (dbg (apply-bias (risk-on) (weights [(position 1 0.4) (position 2 0.6)]))))
