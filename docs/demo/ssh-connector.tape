# Run with: bash docs/demo/record.sh
# setup.sh supplies a fictional HOME, a real binary, and a fake ssh.
Output docs/demo/ssh-connector.gif
Output docs/demo/ssh-connector.mp4
Require ssh-connector
Set Shell bash
Set Width 1100
Set Height 680
Set FontSize 18
Set FontFamily "Menlo"
Set LineHeight 1.1
Set Padding 24
Set Framerate 15
Set CursorBlink false
Set TypingSpeed 90ms
Set Theme {"name":"Showcase Light","background":"#ffffff","foreground":"#172033","black":"#172033","red":"#b91c1c","green":"#166534","yellow":"#92400e","blue":"#1d4ed8","magenta":"#7e22ce","cyan":"#0e7490","white":"#64748b","brightBlack":"#475569","brightRed":"#b91c1c","brightGreen":"#166534","brightYellow":"#92400e","brightBlue":"#1d4ed8","brightMagenta":"#7e22ce","brightCyan":"#0e7490","brightWhite":"#172033","cursor":"#172033","selection":"#dbeafe"}

# Set the prompt in the live shell; VHS can bypass shell startup files.
Hide
Type "export PS1='$ '; unset PROMPT_COMMAND; clear"
Enter
Sleep 200ms
Show

Sleep 1s
Type "ssh-connector"
Enter
Sleep 6s
Type "q"
Enter
Sleep 500ms
Type "clear"
Enter
Sleep 500ms
Type "ssh-connector web"
Enter
Sleep 5s
Type "2"
Enter
Sleep 8s
