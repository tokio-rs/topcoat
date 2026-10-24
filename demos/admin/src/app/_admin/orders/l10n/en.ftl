## Order list

search = Search orders
    .placeholder = Search customers or order numbers
status_filter = Status
    .all = All statuses
sort = Sort by

sort_newest = Newest first
sort_oldest = Oldest first
sort_highest_total = Highest total
sort_lowest_total = Lowest total

no_results = No orders found
    .description = No orders match your search and status.
clear_filters = Clear filters

column_order = Order
column_customer = Customer
column_status = Status
column_items = Items
column_placed = Placed
column_total = Total

item_count =
    { $count ->
        [one] { $count } item
       *[other] { $count } items
    }

just_now = just now
minutes_ago =
    { $count ->
        [one] { $count } minute ago
       *[other] { $count } minutes ago
    }
hours_ago =
    { $count ->
        [one] { $count } hour ago
       *[other] { $count } hours ago
    }
days_ago =
    { $count ->
        [one] { $count } day ago
       *[other] { $count } days ago
    }

status_pending = Pending
status_paid = Paid
status_shipped = Shipped
status_delivered = Delivered
status_cancelled = Cancelled
status_refunded = Refunded

## Order details

# `$id` is the order number.
order_title = Order #{ $id }
# `$date` is when the order was placed.
placed_on = Placed { DATETIME($date, dateStyle: "medium", timeStyle: "short") } UTC

items = Items
unit_count =
    { $count ->
        [one] { $count } unit
       *[other] { $count } units
    }
column_product = Product
column_quantity = Quantity
column_price = Price
column_subtotal = Subtotal
total = Total

customer = Customer
city = City
customer_since = Customer since

# Buttons that move the order to another status.
action_pending = Mark as pending
action_paid = Mark as paid
action_shipped = Mark as shipped
action_delivered = Mark as delivered
action_cancelled = Cancel order
action_refunded = Refund order

# Asks to confirm a status change that cannot be undone. `$status` is the new
# status, such as "cancelled".
confirm_status =
    { $status ->
        [cancelled] Cancel order?
        [refunded] Refund order?
       *[other] Change the status?
    }
final_status = Order #{ $id } cannot move to another status after this.
keep_order = Keep order
