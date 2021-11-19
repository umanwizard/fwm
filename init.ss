(define bindings
  (let ([mod "mod1"])
    (list
     (cons (fwm-parse-key-combo (string-append mod "+h")) (lambda (x) (fwm-navigate x 'left)))
     (cons (fwm-parse-key-combo (string-append mod "+j")) (lambda (x) (fwm-navigate x 'down)))
     (cons (fwm-parse-key-combo (string-append mod "+k")) (lambda (x) (fwm-navigate x 'up)))
     (cons (fwm-parse-key-combo (string-append mod "+l")) (lambda (x) (fwm-navigate x 'right)))
					; (cons (fwm-parse-key-combo (string-append mod "+shift+quotedbl")) fwm-kill-item)
     (cons (fwm-parse-key-combo (string-append mod "+shift+H")) (lambda (x) (fwm-cursor x 'left)))
     (cons (fwm-parse-key-combo (string-append mod "+shift+J")) (lambda (x) (fwm-cursor x 'down)))
     (cons (fwm-parse-key-combo (string-append mod "+shift+K")) (lambda (x) (fwm-cursor x 'up)))
     (cons (fwm-parse-key-combo (string-append mod "+shift+L")) (lambda (x) (fwm-cursor x 'right)))
     (cons (fwm-parse-key-combo (string-append mod "+a")) (lambda (x) (fwm-navigate x 'parent)))
     (cons (fwm-parse-key-combo (string-append mod "+d")) (lambda (x) (fwm-navigate x 'child)))
     (cons (fwm-parse-key-combo (string-append mod "+shift+A")) (lambda (x) (fwm-cursor x 'parent)))
     (cons (fwm-parse-key-combo (string-append mod "+shift+D")) (lambda (x) (fwm-cursor x 'child)))
     (cons (fwm-parse-key-combo (string-append mod "+x")) (lambda (x) (quit)))
					; (cons (fwm-parse-key-combo (string-append mod "+m")) fwm-split-right)
					; (cons (fwm-parse-key-combo (string-append mod "+v")) fwm-split-down)
					;        (cons (fwm-parse-key-combo (string-append mod "+M"))
					;            (lambda (wm)
					;                (fwm-set-cursor wm (fwm-make-cursor (fwm-cursor-item (fwm-get-cursor wm)) 'right))
					;            )
					;        )
					;        (cons (fwm-parse-key-combo (string-append mod "+V")) 
					;            (lambda (wm)
					;                (fwm-set-cursor wm (fwm-make-cursor (fwm-cursor-item (fwm-get-cursor wm)) 'down))
					;            )
					;        )
					;        (cons (fwm-parse-key-combo (string-append mod "+g")) fwm-move)
					;        (cons (fwm-parse-key-combo (string-append mod "+G")) fwm-cursor-to-point)
     (cons (fwm-parse-key-combo (string-append mod "+Return")) (lambda (x) (system "urxvt&")))
					;         (cons (fwm-parse-key-combo (string-append mod "+Return")) (lambda (x) (quit)))
     )
    )
  )

					; TODO - use the cursor for this stuff
(define place-new-window
  (lambda (wm)
    (let* ([point (fwm-get-point wm)]
	   )
      (if (fwm-occupied? wm point)
	  (let* ([container (fwm-nearest-container wm point)]
		 [n_ctr_children (fwm-n-children wm container)])
	    (fwm-make-cursor-into container n_ctr_children) ; Insert at end of the container
	    )
					; The point is unoccupied, so let's insert there.
	  (fwm-make-cursor-before wm point)
	  )
      )
    )
  )
	   
		    

(fwm-run-wm
 (list
  (cons 'bindings  bindings)
  (cons 'place-new-window  place-new-window)
  )
 )
