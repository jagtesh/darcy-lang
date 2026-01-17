(defunion shape
  (circle (radius f64))
  (rect (w f64) (h f64))
  (point))

(defn area [s:shape]
  (match s
    (circle (radius r) (* r r))
    (rect (w w) (h h) (* w h))
    (point 0.0)))

(defn main []
  (print (area (circle 3.0))))
