using Microsoft.AspNetCore.Diagnostics;
using Microsoft.AspNetCore.Mvc;
using Microsoft.AspNetCore.Mvc.RazorPages;

namespace Harbor.Pages;

/// <summary>
/// The 404 and 422 pages, rendered in the shared layout by the status code pages middleware when a
/// request ends with a bare status code, and the 500 page of the exception handler.
/// </summary>
public sealed class ErrorModel : PageModel
{
    public int Code { get; private set; }
    public string Title { get; private set; } = "";
    public string Message { get; private set; } = "";

    public IActionResult OnGet(int code)
    {
        // Only reachable through re-execution; a direct request is just an unknown page.
        if (HttpContext.Features.Get<IStatusCodeReExecuteFeature>() is null
            && HttpContext.Features.Get<IExceptionHandlerPathFeature>() is null)
        {
            return NotFound();
        }
        Code = code;
        (Title, Message) = code switch
        {
            StatusCodes.Status422UnprocessableEntity => (
                "Invalid request",
                "Some of the filters in this link aren't valid. Try browsing from the catalog instead."),
            StatusCodes.Status500InternalServerError => (
                "Something went wrong",
                "Sorry, something went wrong on our side. Please try again in a moment."),
            _ => ("Page not found", "Sorry, we couldn't find the page you're looking for."),
        };
        return Page();
    }
}
