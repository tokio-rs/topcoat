# An amount in US dollars, such as 1234.5.
money = ${ NUMBER($amount, minimumFractionDigits: 2, maximumFractionDigits: 2) }
# An amount in a form field, such as 1234.50. It has no currency symbol or
# thousands separators, so the app can read it back.
money_input = { NUMBER($amount, minimumFractionDigits: 2, maximumFractionDigits: 2, useGrouping: "false") }

role_admin = Admin
role_staff = Staff
role_viewer = Viewer
