(defunion Shape
  (Circle (radius f64))
  (Rect (w f64) (h f64))
  (Point))

(defn area [s:Shape]
  (match s
    (Circle (radius r) (* r r))
    (Rect (w w) (h h) (* w h))
    (Point 0.0)))

(defn main []
  (print (area (Circle 3.0))))
