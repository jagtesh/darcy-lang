; Stage 0: basic types, functions, and control flow
 
(defenum shape
  (circle (radius))
  (rect (w) (h))
  (point))
 
 (defin square [x]
   (* x x))
 
 (defn area [s:shape]
    (case s
      (circle (radius r) (square r))
      (rect (w w) (h h) (* w h))
      (point 0.0)))
 
(defn main []
  (do
    (darcy.io/dbg (area (circle 3.0)))
    (darcy.io/dbg (area (rect 2.0 3.0)))))
