
from cgitb import text
import tkinter
from tkinter import StringVar, filedialog
import os



gui_main = tkinter.Tk()

# 文件夹选择器
def path_folder(in_out):
  
    path_folder = filedialog.askdirectory()
    path_folder = str(os.path.abspath(path_folder))
   
    if in_out == 1:
        global path_in_folder
        path_in_folder = path_folder 
        out_folder_text.config(text=path_folder)
    elif in_out == 0:
        global path_out_folder
        path_out_folder = path_folder
        in_folder_text.config(text=path_folder)        
    else:
        return
   
# 计数器，判断是否点击确定退出

def close_main():
    global chick_quit
    chick_quit = 1
    gui_main.quit()
    

#显示选择的文件夹路径
in_folder_text = tkinter.Label(gui_main,text="in_folder")
out_folder_text = tkinter.Label(gui_main,text="out_folder")
in_folder_text.grid(row=1,column=0)
out_folder_text.grid(row=1,column=2)

#点击按钮选择文件夹
in_folder = tkinter.Button(gui_main,text = "输入",width = 10,command = lambda:path_folder(0))
out_folder = tkinter.Button(gui_main,text = "输出",width = 10,command = lambda:path_folder(1))
in_folder.grid(row=0, column=0)
out_folder.grid(row=0, column=2)

verify = tkinter.Button(gui_main,text="确认",command=close_main)
verify_text = tkinter.Label(gui_main,text="请确认上面显示的路径准确无误?")
verify.grid(row=3,column=1)
verify_text.grid(row=2,column=1)

gui_main.mainloop()


if chick_quit !=1:
    print(chick_quit)
    print("no!!!")
    exit()
    
else:
    print("work!!!")
#print("work!!!")

print(path_in_folder,path_out_folder)