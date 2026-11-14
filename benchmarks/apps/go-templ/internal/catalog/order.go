package catalog

import "time"

// Order is an order from orders.json.
type Order struct {
	ID         int         `json:"id"`
	Number     string      `json:"number"`
	Customer   string      `json:"customer"`
	Email      string      `json:"email"`
	PlacedAt   time.Time   `json:"placed_at"`
	Status     OrderStatus `json:"status"`
	Items      int         `json:"items"`
	TotalCents int         `json:"total_cents"`
	Country    string      `json:"country"`
}

// OrderStatus is the fulfilment state of an order.
type OrderStatus string

const (
	Pending   OrderStatus = "pending"
	Paid      OrderStatus = "paid"
	Shipped   OrderStatus = "shipped"
	Delivered OrderStatus = "delivered"
	Refunded  OrderStatus = "refunded"
	Cancelled OrderStatus = "cancelled"
)

// Label is the display name of the status.
func (s OrderStatus) Label() string {
	switch s {
	case Pending:
		return "Pending"
	case Paid:
		return "Paid"
	case Shipped:
		return "Shipped"
	case Delivered:
		return "Delivered"
	case Refunded:
		return "Refunded"
	case Cancelled:
		return "Cancelled"
	}
	return ""
}

// IsRevenue reports whether the order counts towards revenue.
func (s OrderStatus) IsRevenue() bool {
	return s == Paid || s == Shipped || s == Delivered
}

// OrderSummary holds aggregates over all orders, computed once at startup like a database query
// would.
type OrderSummary struct {
	Count         int
	RevenueCents  int
	AverageCents  int
	RefundedCents int
}

func summarize(orders []Order) OrderSummary {
	var revenue, revenueOrders, refunded int
	for _, order := range orders {
		switch {
		case order.Status.IsRevenue():
			revenue += order.TotalCents
			revenueOrders++
		case order.Status == Refunded:
			refunded += order.TotalCents
		}
	}
	average := 0
	if revenueOrders > 0 {
		average = (revenue + revenueOrders/2) / revenueOrders
	}
	return OrderSummary{
		Count:         len(orders),
		RevenueCents:  revenue,
		AverageCents:  average,
		RefundedCents: refunded,
	}
}
