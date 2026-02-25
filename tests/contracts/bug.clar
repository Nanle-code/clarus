(define-map balances principal uint)
(define-map allowances { owner: principal, spender: principal } uint)

;; deposit — anyone can overwrite another principal's balance
(define-public (deposit)
  (let ((amount (stx-get-balance tx-sender)))
    (map-set balances tx-sender amount)
    (ok true)))

;; withdraw — reentrancy: map-set after stx-transfer? (inside try!)
(define-public (withdraw (amount uint))
  (let ((current-balance (default-to u0 (map-get? balances tx-sender))))
    (asserts! (>= current-balance amount) (err u1))
    (try! (stx-transfer? amount (as-contract tx-sender) tx-sender))
    (map-set balances tx-sender (- current-balance amount))
    (ok true)))

;; withdraw-all — reentrancy: map-delete after stx-transfer?
;;              — unchecked return: stx-transfer? not wrapped in try!
(define-public (withdraw-all)
  (let ((current-balance (default-to u0 (map-get? balances tx-sender))))
    (stx-transfer? current-balance (as-contract tx-sender) tx-sender)
    (map-delete balances tx-sender)
    (ok true)))

;; transfer-from — reentrancy: map-set after stx-transfer?
;;               — integer underflow: (- owner-balance amount) without owner-balance >= amount check
;;               — integer underflow: (- allowed amount) without allowed >= amount check
;;               — unchecked return: stx-transfer? not wrapped in try!
(define-public (transfer-from (owner principal) (amount uint))
  (let (
    (owner-balance (default-to u0 (map-get? balances owner)))
    (allowed (default-to u0 (map-get? allowances { owner: owner, spender: tx-sender })))
  )
    (stx-transfer? amount (as-contract tx-sender) tx-sender)
    (map-set balances owner (- owner-balance amount))
    (map-set allowances { owner: owner, spender: tx-sender } (- allowed amount))
    (ok true)))

;; get-balance — read only, no issues
(define-public (get-balance)
  (ok (default-to u0 (map-get? balances tx-sender))))