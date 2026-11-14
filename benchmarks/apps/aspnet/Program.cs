using System.Globalization;
using System.Net;
using System.Text.Encodings.Web;
using System.Text.Json;
using System.Text.Json.Serialization;
using System.Text.Unicode;
using Harbor.Api;
using Harbor.Catalog;
using Microsoft.Extensions.WebEncoders;

// Formatting never depends on the machine's locale; the formatters also pass the culture explicitly.
CultureInfo.DefaultThreadCurrentCulture = CultureInfo.InvariantCulture;
CultureInfo.DefaultThreadCurrentUICulture = CultureInfo.InvariantCulture;

var builder = WebApplication.CreateBuilder(args);
var config = builder.Configuration;

// The benchmark contract: plain HTTP on 127.0.0.1:$PORT. BENCH_THREADS is applied by `run.sh`.
var port = config.GetValue("PORT", 3000);
builder.WebHost.ConfigureKestrel(kestrel => kestrel.Listen(IPAddress.Loopback, port));

var dataDirectory = config["BENCH_DATA_DIR"]
    ?? Path.Combine(builder.Environment.ContentRootPath, "..", "..", "data");
builder.Services.AddSingleton(Store.Load(dataDirectory));

builder.Services.AddRazorPages();
builder.Services.ConfigureHttpJsonOptions(options =>
{
    options.SerializerOptions.PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower;
    // The web defaults accept numbers in strings; the cart API must reject `"product_id": "1"`.
    options.SerializerOptions.NumberHandling = JsonNumberHandling.Strict;
    options.SerializerOptions.TypeInfoResolverChain.Insert(0, HarborJsonContext.Default);
});
// Razor encodes everything outside Basic Latin as character references by default. Allowing all of
// Unicode keeps names like "Seán" readable in the HTML; markup characters are still escaped.
builder.Services.Configure<WebEncoderOptions>(options =>
    options.TextEncoderSettings = new TextEncoderSettings(UnicodeRanges.All));

var compression = config["BENCH_COMPRESSION"] switch
{
    "on" => true,
    null or "" or "off" => false,
    var value => throw new InvalidOperationException($"invalid BENCH_COMPRESSION \"{value}\""),
};
if (compression)
{
    builder.Services.AddResponseCompression();
}

var app = builder.Build();

// The pipeline of the Razor Pages template (`dotnet new webapp`), plus the benchmark's pieces.
if (!app.Environment.IsDevelopment())
{
    app.UseExceptionHandler("/error/500");
    app.UseHsts();
}
if (compression)
{
    app.UseResponseCompression();
}
// Unknown paths and pages that return a bare 404 or 422 render the error page in the shared layout.
app.UseStatusCodePagesWithReExecute("/error/{0}");
// Without an HTTPS endpoint this does nothing, so plain HTTP on 127.0.0.1 is served as is.
app.UseHttpsRedirection();
app.UseRouting();
app.UseAuthorization();

app.MapStaticAssets();
app.MapRazorPages().WithStaticAssets();
app.MapGet("/health", () => "ok");
app.MapApi();

app.Run();
