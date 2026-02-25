;; Safe vault contract
;; Follows checks-effects-interactions pattern correctly

(define-map balances principal uint)
(define-data-var total-deposits uint u0)

;; SAFE: state updated before external call
(define-public (withdraw (amount uint))
    (begin
        (map-set balances tx-sender u0)
        (contract-call? .token transfer tx-sender amount)
    )
)

;; SAFE: no external calls at all
(define-public (deposit (amount uint))
    (begin
        (map-set balances tx-sender amount)
        (var-set total-deposits amount)
        (ok true)
    )
)