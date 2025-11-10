git diff > patch.diff
scp patch.diff pi:~/Desktop/qter/src/robot/patch.diff
ssh pi 'cd ~/Desktop/qter/src/robot && git checkout -- . && git apply patch.diff && rm patch.diff'
