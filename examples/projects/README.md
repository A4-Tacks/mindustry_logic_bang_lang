# 项目示例 / Project examples
这里是完整可运行的程序, 按用途分目录;
按语法点拆分的小例子在 [../syntax](../syntax), 通用工具库在 [../std](../std)

Runnable whole programs, grouped by purpose.
Single-feature examples live in [../syntax](../syntax), shared libraries in [../std](../std).

## algorithms/ 算法
> [`insert_sort.mdtlbl`](./algorithms/insert_sort.mdtlbl) 插入排序 / Insertion sort<br/>
> [`binary_insert_sort.mdtlbl`](./algorithms/binary_insert_sort.mdtlbl) 二分插入排序 / Binary insertion sort<br/>
> [`shell_sort.mdtlbl`](./algorithms/shell_sort.mdtlbl) Sedgewick 增量希尔排序, 增量预先算成常量 / Sedgewick Shell sort with inlined gaps<br/>
> [`quick_sort.mdtlbl`](./algorithms/quick_sort.mdtlbl) 快排 (基于 0.15.3) / Quick sort (on 0.15.3)<br/>
> [`merge_sort_alternat.mdtlbl`](./algorithms/merge_sort_alternat.mdtlbl) 循环交替归并, 少拷贝 / Alternating merge sort<br/>
> [`pascals_triangle.mdtlbl`](./algorithms/pascals_triangle.mdtlbl) 杨辉三角 / Pascal's triangle<br/>
> [`pascals_triangle.mnd`](./algorithms/pascals_triangle.mnd) 上例的游戏内配套存档 / Saved game companion of the above<br/>

## games/ 游戏与应用
> [`21point.mdtlbl`](./games/21point.mdtlbl) 21 点黑杰克, 内存牌堆加 match / Blackjack with memory deck<br/>
> [`chinese_chess.mdtlbl`](./games/chinese_chess.mdtlbl) 象棋, `High` 控制显示像素高度 / Chinese chess, `High` sets display height<br/>
> [`calculator.mdtlbl`](./games/calculator.mdtlbl) 用较完整 PEG 分析语法的计算器 / Calculator with a near-complete PEG grammar<br/>
> [`ten_step_ten_thousand.mdtlbl`](./games/ten_step_ten_thousand.mdtlbl) 小游戏十步万度复刻 / Ten-step puzzle replica<br/>
> [`luck_draw.mdtlbl`](./games/luck_draw.mdtlbl) 仿原神抽卡, 需 draw print (>146) / Gacha-like draw, needs draw print (>146)<br/>

## display/ 显示与可视化
> [`bezier_curve.mdtlbl`](./display/bezier_curve.mdtlbl) 贝塞尔曲线 / Bezier curve<br/>
> [`sine_superposition.mdtlbl`](./display/sine_superposition.mdtlbl) 正弦波叠加可视化 / Sine superposition<br/>
> [`gravity_simulation.mdtlbl`](./display/gravity_simulation.mdtlbl) 万有引力星球模拟, 无碰撞 / Gravity simulation, no collision<br/>
> [`pid_view.mdtlbl`](./display/pid_view.mdtlbl) 动态调试 PID 参数与曲线 / Live PID params and curves<br/>
> [`display_numbers.mdtlbl`](./display/display_numbers.mdtlbl) 高效显数 / Efficient number display<br/>
> [`sorter_display_num.mdtlbl`](./display/sorter_display_num.mdtlbl) 分类器显数, 好读易扩展 / Sorter display, readable and extensible<br/>
> [`number_formats.mdtlbl`](./display/number_formats.mdtlbl) 指定位读数, 以 Dec Hex Oct Bin 输出 / Read bits, print Dec Hex Oct Bin<br/>
> [`say_message.mdtlbl`](./display/say_message.mdtlbl) 信息板聊天记录并按时间淘汰 / Message board with age eviction<br/>
> [`item_ranking_list.mdtlbl`](./display/item_ranking_list.mdtlbl) 物品数量排行榜带动画 / Item ranking list with animation<br/>
> [`mini_power_meter.mdtlbl`](./display/mini_power_meter.mdtlbl) 迷你电力表 / Mini power meter<br/>

## unit_control/ 单位控制
> [`one_unit.mdtlbl`](./unit_control/one_unit.mdtlbl) 简单单控 / Simple single-unit control<br/>
> [`mult_unit_control_template.mdtlbl`](./unit_control/mult_unit_control_template.mdtlbl) 多控模板, 按控制状态绑定 / Multicontrol template binding by control state<br/>
> [`item_transport_beans.mdtlbl`](./unit_control/item_transport_beans.mdtlbl) 小搬运豆 / Item hauling bean<br/>
> [`item_transport_beans_extend.mdtlbl`](./unit_control/item_transport_beans_extend.mdtlbl) 多功能搬运豆 / Extended hauling bean<br/>
> [`item_transport_beans_extend_mult_units.mdtlbl`](./unit_control/item_transport_beans_extend_mult_units.mdtlbl) 多功能搬运豆多单位版 / Extended bean, multi unit<br/>
> [`unit_count.mdtlbl`](./unit_control/unit_count.mdtlbl) 统计所有单位 / Count units by type<br/>
> [`all_shoot.mdtlbl`](./unit_control/all_shoot.mdtlbl) 炮台跟随首个链接炮台开火 / Turrets follow the first linked turret<br/>

## utilities/ 世界机制运用
> [`conveyor_logic_gate.mdtlbl`](./utilities/conveyor_logic_gate.mdtlbl) 传送带启停做逻辑门 / Logic gates from conveyor enable<br/>
> [`timer.mdtlbl`](./utilities/timer.mdtlbl) 侦测当前逻辑实际执行速度 / Measure the processor's real speed<br/>
