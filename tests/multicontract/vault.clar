;; vault contract — vulnerable cross-contract reentrancy example
(define-map balances principal uint)
(define-data-var total-staked uint u0)

;; VULNERABLE: calls token contract then updates state
(define-public (withdraw (amount uint))
    (let ((current-balance (default-to u0 (map-get? balances tx-sender))))
        (asserts! (>= current-balance amount) (err u1))
        (contract-call? .token transfer amount (as-contract tx-sender) tx-sender)
        (map-set balances tx-sender (- current-balance amount))
        (ok true)))

;; VULNERABLE: calls rewards then updates state
(define-public (claim-and-withdraw (amount uint))
    (let ((current-balance (default-to u0 (map-get? balances tx-sender))))
        (contract-call? .rewards distribute tx-sender)
        (var-set total-staked (- current-balance amount))
        (ok true)))

;; SAFE: state updated before external call
(define-public (deposit (amount uint))
    (begin
        (asserts! (is-eq tx-sender contract-caller) (err u1))
        (map-set balances tx-sender amount)
        (var-set total-staked amount)
        (ok true)))