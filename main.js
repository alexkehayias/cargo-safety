var child_process = require('child_process');

exports.handler = function(event, context) {
  console.log(event);

  // spawn a child process to run the binary
  var proc = child_process.spawn('./target/release/harbor', [event["git-url"]], {stdio: "inherit"});

  proc.on('close', function(code) {
    if(code !== 0) {
      return context.done(new Error("Process exited with non-zero status code"));
    }

    context.done(null);
  });
}
