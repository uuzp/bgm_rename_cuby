
import tkinter
from tkinter import messagebox,filedialog
import os

from setuptools import Command

#!/usr/bin/python3
 

top = tkinter.Tk()
# 进入消息循环
# top.mainloop()

""" link_sf = filedialog.askdirectory()

link_sf = str(os.path.abspath(link_sf)) """
eee = 0 

def op1():
    messagebox.showinfo( "Hello Python", "Hello Runoob")
    
w = tkinter.Button(top,text = "源文件夹",width=10,command=op1)

s = tkinter.Button(top,text = "输出文件夹",width=10,command=op1)
w.grid(row=0, column=0)
s.grid(row=0, column=2)

q = tkinter.Label(top,text="video!",width=10)
q2 = tkinter.Label(top,text="video!",width=10)
q3 = tkinter.Label(top,text="ture?",width=10)

q.grid(row=1,column=0)
q2.grid(row=1,column=2)
q3.grid(row=1,column=1)

def close_top(): 
    global eee
    eee = 1
    top.quit()
    

o3 = tkinter.Button(top,text="确认",command=close_top)
o3.grid(row=2,column=1)

#s.pack()
#w.pack()
top.mainloop()
if eee !=1:
    print(eee)
    print("no!!!")
    exit()
    
else:
    print("work!!!")
#print("work!!!")