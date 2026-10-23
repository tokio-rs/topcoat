# An amount in US dollars, such as 1234.5.
money = ${ NUMBER($amount, minimumFractionDigits: 2, maximumFractionDigits: 2) }

# The items on one page of a list, such as "21-40 of 45".
pagination-summary = { $first }-{ $last } of { $count }
pagination-previous = Previous
pagination-next = Next

role-admin = Admin
role-staff = Staff
role-viewer = Viewer
