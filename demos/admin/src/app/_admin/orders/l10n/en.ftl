## Order list

search = Search orders
    .placeholder = Search customers or order numbers
status-filter = Status
    .all = All statuses
sort = Sort by

sort-newest = Newest first
sort-oldest = Oldest first
sort-highest-total = Highest total
sort-lowest-total = Lowest total

no-results = No orders found
    .description = No orders match your search and status.
clear-filters = Clear filters

column-order = Order
column-customer = Customer
column-status = Status
column-items = Items
column-placed = Placed
column-total = Total

item-count =
    { $count ->
        [one] { $count } item
       *[other] { $count } items
    }

just-now = just now
minutes-ago =
    { $count ->
        [one] { $count } minute ago
       *[other] { $count } minutes ago
    }
hours-ago =
    { $count ->
        [one] { $count } hour ago
       *[other] { $count } hours ago
    }
days-ago =
    { $count ->
        [one] { $count } day ago
       *[other] { $count } days ago
    }

status-pending = Pending
status-paid = Paid
status-shipped = Shipped
status-delivered = Delivered
status-cancelled = Cancelled
status-refunded = Refunded

## Order details

# `$id` is the order number.
order-title = Order #{ $id }
# `$date` is when the order was placed, such as "Oct 12, 2026 at 14:05 UTC".
placed-on = Placed { $date }

items = Items
unit-count =
    { $count ->
        [one] { $count } unit
       *[other] { $count } units
    }
column-product = Product
column-quantity = Quantity
column-price = Price
column-subtotal = Subtotal
total = Total

customer = Customer
city = City
customer-since = Customer since

# Buttons that move the order to another status.
action-pending = Mark as pending
action-paid = Mark as paid
action-shipped = Mark as shipped
action-delivered = Mark as delivered
action-cancelled = Cancel order
action-refunded = Refund order

# Asks to confirm a status change that cannot be undone. `$status` is the new
# status, such as "cancelled".
confirm-status =
    { $status ->
        [cancelled] Cancel order?
        [refunded] Refund order?
       *[other] Change the status?
    }
final-status = Order #{ $id } cannot move to another status after this.
keep-order = Keep order
