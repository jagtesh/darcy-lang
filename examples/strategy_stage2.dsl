; Stage 2: extern types + functions, regime-based adjustments
; Note: extern items are not codegen'd; they must exist in the host Rust crate.

(extern "Candle" (defstruct candle
  (ts i64)
  (close f64)
  (volume f64)))

(defunion regime
  (bull)
  (bear)
  (sideways))

(extern (defn sma [values:vec<f64> period:i32] vec<f64>))
(extern (defn regime-of [values:vec<f64>] regime))

(defn risk-adjust [r:regime returns:vec<f64>]
  (match r
    (bull (* returns 1.2))
    (bear (* returns 0.5))
    (sideways returns)))

(defn main []
  (dbg (risk-adjust (regime-of [0.01 0.02 -0.01]) (sma [0.01 0.02 -0.01] 2))))
