
# Roteiro da demo do README; rode por assets/demo.sh, nunca direto.
Output assets/demo.gif

Require get-env

Set Shell bash
Set Theme "Catppuccin Mocha"
Set FontSize 22
Set Width 1200
Set Height 640
Set Padding 28
Set LineHeight 1.25
Set TypingSpeed 70ms

Hide
Type "clear"
Enter
Show

Type "get-env openai_key"
Sleep 400ms
Enter
Sleep 2.5s
Screenshot assets/card.png
Sleep 500ms

Hide
Type "clear"
Enter
Show

Type "get-env aws"
Sleep 400ms
Enter
Sleep 1.5s
Down
Sleep 600ms
Down
Sleep 600ms
Type "v"
Sleep 1.5s
Screenshot assets/table.png
Sleep 800ms
Type "/"
Sleep 400ms
Backspace 3
Sleep 300ms
Type "token"
Sleep 1.2s
Enter
Sleep 400ms
Type "n"
Sleep 2s
Type "q"
Sleep 800ms

