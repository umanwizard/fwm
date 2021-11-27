(define at-point
  (lambda (f)
    (lambda (wm)
      (f wm (fwm-get-point wm)))))

(define terminal "sakura")
(define exec
  (lambda (cmd)
    (system (string-append cmd "&"))))

(system "xmodmap ~/.Xmodmap")
(exec "xscreensaver -no-splash&")
(system "xsetroot -mod 4 10")

(define bindings
  (let ([mod "mod3"])
    (list
     (cons (fwm-parse-key-combo (string-append mod "+h")) (lambda (x) (fwm-navigate x '(Planar . Left))))
     (cons (fwm-parse-key-combo (string-append mod "+j")) (lambda (x) (fwm-navigate x '(Planar . Down))))
     (cons (fwm-parse-key-combo (string-append mod "+k")) (lambda (x) (fwm-navigate x '(Planar . Up))))
     (cons (fwm-parse-key-combo (string-append mod "+l")) (lambda (x) (fwm-navigate x '(Planar . Right))))
     (cons (fwm-parse-key-combo (string-append mod "+shift+apostrophe")) (at-point fwm-kill-item-at))
     (cons (fwm-parse-key-combo (string-append mod "+apostrophe")) (at-point fwm-kill-client-at))
     (cons (fwm-parse-key-combo (string-append mod "+shift+h")) (lambda (x) (fwm-cursor x '(Planar . Left))))
     (cons (fwm-parse-key-combo (string-append mod "+shift+j")) (lambda (x) (fwm-cursor x '(Planar . Down))))
     (cons (fwm-parse-key-combo (string-append mod "+shift+k")) (lambda (x) (fwm-cursor x '(Planar . Up))))
     (cons (fwm-parse-key-combo (string-append mod "+shift+l")) (lambda (x) (fwm-cursor x '(Planar . Right))))
     (cons (fwm-parse-key-combo (string-append mod "+a")) (lambda (x) (fwm-navigate x 'Parent)))
     (cons (fwm-parse-key-combo (string-append mod "+d")) (lambda (x) (fwm-navigate x 'Child)))
     (cons (fwm-parse-key-combo (string-append mod "+shift+a")) (lambda (x) (fwm-cursor x 'Parent)))
     (cons (fwm-parse-key-combo (string-append mod "+shift+d")) (lambda (x) (fwm-cursor x 'Child)))
     (cons (fwm-parse-key-combo (string-append mod "+shift+period")) (lambda (x) (quit)))
     (cons (fwm-parse-key-combo (string-append mod "+p")) (lambda (x) (let ([layout (fwm-dump-layout x)]) (display layout) (newline)  )))
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
     (cons (fwm-parse-key-combo (string-append mod "+Return")) (lambda (x) (exec terminal)))
     (cons (fwm-parse-key-combo (string-append mod "+e")) (lambda (x) (exec "rofi -show run")))
     (cons (fwm-parse-key-combo (string-append mod "+q")) (lambda (x) (exec "xscreensaver-command -lock")))
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
	   
(define focus-if-window
  (lambda (wm point)
    (when (eq? (car point) 'Window)
      (fwm-set-focus wm (list (cdr point))))))
		    

(fwm-run-wm
 (list
  (cons 'bindings  bindings)
  (cons 'place-new-window  place-new-window)
  (cons 'on-point-changed focus-if-window)
  )
 )
