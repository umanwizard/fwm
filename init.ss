(define bindings
  (let ([mod "mod1"])
    (list
     (cons (fwm-parse-key-combo (string-append mod "+h")) (lambda (x) (fwm-navigate x '(Planar . Left))))
     (cons (fwm-parse-key-combo (string-append mod "+j")) (lambda (x) (fwm-navigate x '(Planar . Down))))
     (cons (fwm-parse-key-combo (string-append mod "+k")) (lambda (x) (fwm-navigate x '(Planar . Up))))
     (cons (fwm-parse-key-combo (string-append mod "+l")) (lambda (x) (fwm-navigate x '(Planar . Right))))
					; (cons (fwm-parse-key-combo (string-append mod "+shift+quotedbl")) fwm-kill-item)
     (cons (fwm-parse-key-combo (string-append mod "+shift+H")) (lambda (x) (fwm-cursor x '(Planar . Left))))
     (cons (fwm-parse-key-combo (string-append mod "+shift+J")) (lambda (x) (fwm-cursor x '(Planar . Down))))
     (cons (fwm-parse-key-combo (string-append mod "+shift+K")) (lambda (x) (fwm-cursor x '(Planar . Up))))
     (cons (fwm-parse-key-combo (string-append mod "+shift+L")) (lambda (x) (fwm-cursor x '(Planar . Right))))
     (cons (fwm-parse-key-combo (string-append mod "+a")) (lambda (x) (fwm-navigate x 'Parent)))
     (cons (fwm-parse-key-combo (string-append mod "+d")) (lambda (x) (fwm-navigate x 'Child)))
     (cons (fwm-parse-key-combo (string-append mod "+shift+A")) (lambda (x) (fwm-cursor x 'Parent)))
     (cons (fwm-parse-key-combo (string-append mod "+shift+D")) (lambda (x) (fwm-cursor x 'Child)))
     (cons (fwm-parse-key-combo (string-append mod "+x")) (lambda (x) (quit)))
					; (cons (fwm-parse-key-combo (string-append mod "+m")) fwm-split-Right)
					; (cons (fwm-parse-key-combo (string-append mod "+v")) fwm-split-Down)
					;        (cons (fwm-parse-key-combo (string-append mod "+M"))
					;            (lambda (wm)
					;                (fwm-set-cursor wm (fwm-make-cursor (fwm-cursor-item (fwm-get-cursor wm)) '(Planar . Right)))
					;            )
					;        )
					;        (cons (fwm-parse-key-combo (string-append mod "+V")) 
					;            (lambda (wm)
					;                (fwm-set-cursor wm (fwm-make-cursor (fwm-cursor-item (fwm-get-cursor wm)) '(Planar . Down)))
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
	  (cons 'Replace point)
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
