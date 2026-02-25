;; Vulnerable vault contract
;; Demonstrates reentrancy via contract-call? before state update

(define-map balances principal uint)
(define-data-var total-deposits uint u0)

;; VULNERABLE: external call happens before state is updated
(define-public (withdraw (amount uint))
    (begin
        (contract-call? .token transfer tx-sender amount)
        (map-set balances tx-sender u0)
    )
)

;; VULNERABLE: var-set after external call
(define-public (claim-rewards)
    (begin
        (contract-call? .rewards distribute tx-sender)
        (var-set total-deposits u0)
    )
)

;; SAFE: state updated before external call
(define-public (safe-withdraw (amount uint))
    (begin
        (map-set balances tx-sender u0)
        (contract-call? .token transfer tx-sender amount)
    )
)