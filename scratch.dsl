(require [darcy.fmt :refer [println]])
(require [darcy.math :as math])

(defenum games
	(strategy)
	(action)
	(racing))

(defn choose-game [game]
	(case game
		(strategy (println "strategy!"))
		(action (println "action!"))
		(racing (println "racing!"))))

(defn call-me-ishmael [name]
	(if (= name "ishmael")
		(println "you get me!")
		(println "what's wrong with you!")))

(defn fact [n]
	(let [first 0 second 1]
		(for i (range 0 (/ n 2))
			(let! first (+ first second))
			(let! second (+ first second))
			(println first)
			;; if this is an even number, we can print second
			(if (= (mod n 2) 0)
				(println second)))))

(defn main []
	(choose-game (racing))
	(call-me-ishmael "ishmael ")
	(fact 2))
